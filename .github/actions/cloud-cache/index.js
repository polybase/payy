const core = require("@actions/core");
const { Storage } = require("@google-cloud/storage");
const {
  S3Client,
  HeadObjectCommand,
  GetObjectCommand,
} = require("@aws-sdk/client-s3");
const fs = require("fs").promises;
const rawFs = require("fs");
const path = require("path");
const cp = require("child_process");
const os = require("os");
const util = require("util");
const exec = util.promisify(cp.exec);
function sleep(ms) {
  return new Promise((res) => setTimeout(res, ms));
}
function expandTilde(inputPath) {
  if (inputPath.charAt(0) === "~") {
    const home = os.homedir();
    return home + inputPath.slice(1);
  }
  return inputPath;
}
async function run() {
  try {
    // Get inputs
    const provider = core.getInput("provider").toLowerCase();
    const bucketName = core.getInput("bucket");
    let prefix = core.getInput("prefix");
    const primaryKeyInput = core.getInput("primary-key");
    const primaryKeyGenerator = core.getInput("primary-key-generator");
    const fallbackKeysInput = core.getInput("fallback-keys");
    const fallbackKeyGenerator = core.getInput("fallback-key-generator");
    const localPathInput = core.getInput("local-path");
    const skipDownload = core.getInput("skip-download") === "true";
    const localPath = path.resolve(process.cwd(), expandTilde(localPathInput));
    if (!primaryKeyInput && !primaryKeyGenerator) {
      throw new Error(
        'Either "primary-key" or "primary-key-generator" must be provided'
      );
    }
    const retryAttempts = parseInt(core.getInput("retries") || "3", 10);
    const retryBaseDelayMs = parseInt(
      core.getInput("retry-delay-ms") || "1000",
      10
    );
    const downloadPartCount = Math.max(
      1,
      parseInt(core.getInput("download-part-count") || "128", 10)
    );
    const downloadConcurrencyInput = parseInt(
      core.getInput("download-concurrency") || "32",
      10
    );
    // Save state for post
    core.saveState("provider", provider);
    core.saveState("bucketName", bucketName);
    core.saveState("local-path", localPath);
    let gcsStorage, gcsBucket, s3Client;
    
    // Only initialize storage clients if we're actually going to use them
    // When skip-download is true, we don't need credentials for downloading
    const needsCredentials = !skipDownload;
    
    if (provider === "gcs") {
      const base64Key = core.getInput("gcs-key");
      if (!base64Key) {
        if (needsCredentials) {
          throw new Error("gcs-key is required when skip-download is false");
        }
        core.warning("GCS key not provided. Cache operations will be skipped.");
        core.setOutput("cache-hit", "false");
        core.setOutput("restored", "false");
        core.setOutput("path", localPath);
        return;
      }
      core.saveState("base64Key", base64Key);
      // Decode base64 key to JSON
      const jsonKeyString = Buffer.from(base64Key, "base64").toString("utf-8");
      let credentials;
      try {
        credentials = JSON.parse(jsonKeyString);
      } catch (err) {
        throw new Error("Invalid base64 encoded JSON key");
      }
      // Initialize GCS client
      gcsStorage = new Storage({ credentials });
      gcsBucket = gcsStorage.bucket(bucketName);
    } else if (provider === "s3") {
      const s3AccessKey = core.getInput("s3-access-key");
      const s3SecretKey = core.getInput("s3-secret-key");
      const s3Endpoint = core.getInput("s3-endpoint");
      const s3Region = core.getInput("s3-region");

      if (!s3AccessKey || !s3SecretKey) {
        if (needsCredentials) {
          throw new Error("s3-access-key and s3-secret-key are required when skip-download is false");
        }
        core.warning("S3 credentials not provided. Cache operations will be skipped.");
        core.setOutput("cache-hit", "false");
        core.setOutput("restored", "false");
        core.setOutput("path", localPath);
        return;
      }

      core.saveState("s3AccessKey", s3AccessKey);
      core.saveState("s3SecretKey", s3SecretKey);
      core.saveState("s3Endpoint", s3Endpoint);
      core.saveState("s3Region", s3Region);
      // Initialize S3 client
      const s3Config = {
        region: s3Region || "auto",
        credentials: {
          accessKeyId: s3AccessKey,
          secretAccessKey: s3SecretKey,
        },
        forcePathStyle: true,
      };
      if (s3Endpoint) {
        s3Config.endpoint = s3Endpoint;
      }
      s3Client = new S3Client(s3Config);
    } else {
      throw new Error(`Unsupported provider: ${provider}`);
    }
    // Normalize prefix
    if (prefix && !prefix.endsWith("/")) {
      prefix += "/";
    }
    if (!prefix) {
      prefix = "";
    }
    core.saveState("prefix", prefix);
    // Determine primary key (without extension)
    let primary = "";
    if (primaryKeyGenerator) {
      // Create temp file for PRIMARY_KEY
      const primaryKeyPath = path.join(
        os.tmpdir(),
        `primary_key_${process.pid}.txt`
      );
      await fs.writeFile(primaryKeyPath, "");
      // Create temp script file
      const scriptPath = path.join(
        os.tmpdir(),
        `primary_gen_${process.pid}.sh`
      );
      const scriptContent = `#!/bin/bash\n${primaryKeyGenerator}`;
      await fs.writeFile(scriptPath, scriptContent);
      await fs.chmod(scriptPath, 0o755);
      // Execute the script
      await new Promise((resolve, reject) => {
        const child = cp.execFile(scriptPath, {
          env: { ...process.env, PRIMARY_KEY: primaryKeyPath },
        });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (data) => {
          stdout += data;
        });
        child.stderr.on("data", (data) => {
          stderr += data;
        });
        child.on("close", (code) => {
          if (code !== 0) {
            reject(
              new Error(
                `Primary script exited with code ${code}. Stderr: ${stderr}`
              )
            );
          } else {
            core.info(`Primary script output: ${stdout}`);
            if (stderr) core.warning(`Primary script stderr: ${stderr}`);
            resolve();
          }
        });
      });
      // Read generated primary key
      const primaryContent = await fs.readFile(primaryKeyPath, "utf-8");
      const primaryLines = primaryContent
        .split("\n")
        .map((k) => k.trim())
        .filter((k) => k.length > 0);
      if (primaryLines.length !== 1) {
        throw new Error("Primary key generator must produce exactly one key");
      }
      primary = primaryLines[0];
      // Clean up temp files
      await fs.unlink(scriptPath).catch(() => {});
      await fs.unlink(primaryKeyPath).catch(() => {});
    } else {
      primary = primaryKeyInput.trim();
      if (!primary) {
        throw new Error("Primary key is empty");
      }
    }
    core.saveState("primaryKey", primary);
    // Optionally skip restore phase
    if (skipDownload) {
      core.setOutput("cache-hit", false);
      core.saveState("cacheHit", false);
      core.setOutput("path", localPath);
      core.setOutput("restored", false);
      console.log("skip-download parameter is true, skipping cache restore");
      return;
    }
    // Determine fallback keys (without extension)
    let fallbacks = [];
    if (fallbackKeyGenerator) {
      // Create temp file for FALLBACK_KEYS
      const fallbackKeysPath = path.join(
        os.tmpdir(),
        `fallback_keys_${process.pid}.txt`
      );
      await fs.writeFile(fallbackKeysPath, "");
      // Create temp script file
      const scriptPath = path.join(
        os.tmpdir(),
        `fallback_gen_${process.pid}.sh`
      );
      const scriptContent = `#!/bin/bash\n${fallbackKeyGenerator}`;
      await fs.writeFile(scriptPath, scriptContent);
      await fs.chmod(scriptPath, 0o755);
      // Execute the script
      await new Promise((resolve, reject) => {
        const child = cp.execFile(scriptPath, {
          env: { ...process.env, FALLBACK_KEYS: fallbackKeysPath },
        });
        let stdout = "";
        let stderr = "";
        child.stdout.on("data", (data) => {
          stdout += data;
        });
        child.stderr.on("data", (data) => {
          stderr += data;
        });
        child.on("close", (code) => {
          if (code !== 0) {
            reject(
              new Error(
                `Fallback script exited with code ${code}. Stderr: ${stderr}`
              )
            );
          } else {
            core.info(`Fallback script output: ${stdout}`);
            if (stderr) core.warning(`Fallback script stderr: ${stderr}`);
            resolve();
          }
        });
      });
      // Read generated fallback keys
      const fallbacksContent = await fs.readFile(fallbackKeysPath, "utf-8");
      fallbacks = fallbacksContent
        .split("\n")
        .map((k) => k.trim())
        .filter((k) => k.length > 0);
      // Clean up temp files
      await fs.unlink(scriptPath).catch(() => {});
      await fs.unlink(fallbackKeysPath).catch(() => {});
    } else if (fallbackKeysInput) {
      fallbacks = fallbackKeysInput
        .split(",")
        .map((k) => k.trim())
        .filter((k) => k.length > 0);
    }
    // Add extension to all keys
    const primaryWithExt = primary + ".tar.zst";
    const fallbacksWithExt = fallbacks.map((k) => k + ".tar.zst");
    const keys = [primaryWithExt, ...fallbacksWithExt];
    if (keys.length === 0) {
      throw new Error("No keys provided or generated");
    }
    // Check existence in parallel
    const existsPromises = keys.map(async (key) => {
      const fullPath = prefix + key;
      if (provider === "gcs") {
        const [exists] = await gcsBucket.file(fullPath).exists();
        return exists;
      } else if (provider === "s3") {
        try {
          await s3Client.send(
            new HeadObjectCommand({ Bucket: bucketName, Key: fullPath })
          );
          return true;
        } catch (err) {
          if (err.name === "NotFound") {
            return false;
          }
          throw err;
        }
      }
    });
    const existsResults = await Promise.all(existsPromises);
    // Find the first existing key in order
    let selectedIndex = -1;
    for (let i = 0; i < existsResults.length; i++) {
      if (existsResults[i]) {
        selectedIndex = i;
        break;
      }
    }
    const cacheHit = selectedIndex === 0;
    core.setOutput("cache-hit", cacheHit);
    core.saveState("cacheHit", cacheHit);
    let restored = false;
    if (selectedIndex !== -1) {
      // Download and extract the selected archive
      const selectedKey = keys[selectedIndex];
      const fullPath = prefix + selectedKey;
      const tempDir = os.tmpdir();
      const tempArchive = path.join(tempDir, `cache_${process.pid}.tar.zst`);
      const tempTar = path.join(tempDir, `cache_${process.pid}.tar`);
      let fileSize = 0;
      if (provider === "gcs") {
        const file = gcsBucket.file(fullPath);
        const [metadata] = await file.getMetadata();
        fileSize = Number(metadata.size);
      } else if (provider === "s3") {
        const head = await s3Client.send(
          new HeadObjectCommand({ Bucket: bucketName, Key: fullPath })
        );
        fileSize = head.ContentLength;
      }
      if (fileSize === 0) {
        await fs.writeFile(tempArchive, "");
      } else {
        await fs.writeFile(tempArchive, "");
        await fs.truncate(tempArchive, fileSize);
        // Derive part size from desired part count
        const partSize = Math.max(1, Math.ceil(fileSize / downloadPartCount));
        const numParts = Math.max(1, Math.ceil(fileSize / partSize));
        const downloadConcurrency = Math.max(
          1,
          Math.min(downloadConcurrencyInput, numParts)
        );
        console.log(
          `Downloading ${fileSize} bytes in ${numParts} parts (concurrency=${downloadConcurrency})`
        );
        async function downloadPart(start, end) {
          for (let attempt = 1; attempt <= retryAttempts; attempt++) {
            let readStream;
            let writeStream;
            try {
              if (provider === "gcs") {
                const file = gcsBucket.file(fullPath);
                readStream = file.createReadStream({ start, end });
              } else if (provider === "s3") {
                const getCommand = new GetObjectCommand({
                  Bucket: bucketName,
                  Key: fullPath,
                  Range: `bytes=${start}-${end}`,
                });
                const response = await s3Client.send(getCommand);
                readStream = response.Body;
              }
              writeStream = rawFs.createWriteStream(tempArchive, {
                flags: "r+",
                start,
              });
              await new Promise((resolve, reject) => {
                const onError = async (err) => {
                  cleanup();
                  reject(err);
                };
                const onFinish = () => {
                  cleanup();
                  resolve();
                };
                const cleanup = () => {
                  if (readStream) {
                    readStream.removeListener("error", onError);
                  }
                  if (writeStream) {
                    writeStream.removeListener("error", onError);
                    writeStream.removeListener("finish", onFinish);
                  }
                };
                writeStream.on("finish", onFinish);
                writeStream.on("error", onError);
                readStream.on("error", onError);
                readStream.pipe(writeStream);
              });
              return; // success
            } catch (err) {
              if (attempt >= retryAttempts) {
                core.error(
                  `Download part ${start}-${end} failed after ${attempt} attempts: ${err.message}`
                );
                throw err;
              }
              const backoff =
                retryBaseDelayMs * Math.pow(2, attempt - 1) +
                Math.floor(Math.random() * retryBaseDelayMs);
              core.warning(
                `Download part ${start}-${end} attempt ${attempt} failed: ${err.message}. Retrying in ${backoff}ms`
              );
              await sleep(backoff);
            }
          }
        }
        let nextIndex = 0;
        async function worker() {
          while (true) {
            const i = nextIndex++;
            if (i >= numParts) return;
            const start = i * partSize;
            const end = Math.min(start + partSize - 1, fileSize - 1);
            if (start > end) continue;
            await downloadPart(start, end);
          }
        }
        const workers = [];
        for (let w = 0; w < downloadConcurrency; w++) {
          workers.push(worker());
        }
        await Promise.all(workers);
      }
      // Decompress
      await exec(`zstd -d ${tempArchive} -o ${tempTar}`);
      // Create local directory if needed
      await fs.mkdir(localPath, { recursive: true });
      // Extract
      await exec(`tar -xf ${tempTar} -C ${localPath}`);
      // Clean up temps
      await fs.unlink(tempArchive).catch(() => {});
      await fs.unlink(tempTar).catch(() => {});
      restored = true;
      console.log(`Restored cache from ${fullPath} to ${localPath}`);
    } else {
      console.log("Cache miss: no matching keys found");
    }
    // Set outputs
    core.setOutput("path", localPath);
    core.setOutput("restored", restored);
  } catch (error) {
    core.setFailed(error.message);
  }
}
run();
