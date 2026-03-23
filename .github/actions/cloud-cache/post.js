const core = require("@actions/core");
const { Storage } = require("@google-cloud/storage");
const {
  S3Client,
  PutObjectCommand,
  CreateMultipartUploadCommand,
  UploadPartCommand,
  CompleteMultipartUploadCommand,
  AbortMultipartUploadCommand,
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
async function run() {
  try {
    const skipUpload = core.getInput("skip-upload") === "true";
    if (skipUpload) {
      console.log("skip-upload parameter is true, skipping cache upload");
      return;
    }
    const cacheHit = core.getState("cacheHit") === "true";
    if (cacheHit) {
      console.log("Exact cache hit on primary key, skipping upload");
      return;
    }
    const localPath = core.getState("local-path");
    // Check if directory exists and is not empty
    let dirExists = false;
    let isNotEmpty = false;
    try {
      await fs.access(localPath);
      dirExists = true;
      const files = await fs.readdir(localPath);
      isNotEmpty = files.length > 0;
    } catch {
      // Dir doesn't exist or error
    }
    if (!dirExists || !isNotEmpty) {
      console.log(
        "Local directory does not exist or is empty, skipping upload"
      );
      return;
    }
    const primaryKey = core.getState("primaryKey");
    const prefix = core.getState("prefix");
    const bucketName = core.getState("bucketName");
    const provider = core.getState("provider");
    const retryAttempts = parseInt(core.getInput("retries") || "3", 10);
    const retryBaseDelayMs = parseInt(
      core.getInput("retry-delay-ms") || "1000",
      10
    );
    const uploadConcurrencyInput = parseInt(
      core.getInput("upload-concurrency") || "32",
      10
    );
    let gcsStorage, gcsBucket, s3Client;
    if (provider === "gcs") {
      const base64Key = core.getState("base64Key");
      if (!base64Key) {
        console.log("GCS key not available, skipping cache upload");
        return;
      }
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
      const s3AccessKey = core.getState("s3AccessKey");
      const s3SecretKey = core.getState("s3SecretKey");
      if (!s3AccessKey || !s3SecretKey) {
        console.log("S3 credentials not available, skipping cache upload");
        return;
      }
      const s3Endpoint = core.getState("s3Endpoint");
      const s3Region = core.getState("s3Region");
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
    }
    const primaryWithExt = primaryKey + ".tar.zst";
    const fullPath = prefix + primaryWithExt;
    const tempDir = os.tmpdir();
    const tempTar = path.join(tempDir, `cache_${process.pid}.tar`);
    const tempArchive = path.join(tempDir, `cache_${process.pid}.tar.zst`);
    // Pack directory
    await exec(`tar -cf ${tempTar} -C ${localPath} .`);
    // Compress with fast preset
    await exec(`zstd --long ${tempTar} -o ${tempArchive}`);
    // Upload
    const stats = await fs.stat(tempArchive);
    const fileSize = stats.size;
    if (provider === "gcs") {
      // google-cloud/storage uses resumable uploads by default and has built-in retry.
      // We call once; internal logic will resume on transient failures.
      await gcsBucket.upload(tempArchive, { destination: fullPath, resumable: true });
    } else if (provider === "s3") {
      if (fileSize < 5 * 1024 * 1024) {
        // Use single PutObject for small files (<5MB)
        for (let attempt = 1; attempt <= retryAttempts; attempt++) {
          try {
            await s3Client.send(
              new PutObjectCommand({
                Bucket: bucketName,
                Key: fullPath,
                Body: rawFs.createReadStream(tempArchive),
              })
            );
            break; // success
          } catch (err) {
            if (attempt >= retryAttempts) {
              core.error(
                `PutObject failed after ${attempt} attempts: ${err.message}`
              );
              throw err;
            }
            const backoff =
              retryBaseDelayMs * Math.pow(2, attempt - 1) +
              Math.floor(Math.random() * retryBaseDelayMs);
            core.warning(
              `PutObject attempt ${attempt} failed: ${err.message}. Retrying in ${backoff}ms`
            );
            await sleep(backoff);
          }
        }
      } else {
        // Use multipart for larger files
        const PART_SIZE = 200 * 1024 * 1024; // 100MB parts for parallelism (<<5GB)
        const createRes = await s3Client.send(
          new CreateMultipartUploadCommand({
            Bucket: bucketName,
            Key: fullPath,
          })
        );
        const uploadId = createRes.UploadId;
        try {
          const numParts = Math.ceil(fileSize / PART_SIZE);
          const uploadConcurrency = Math.max(
            1,
            Math.min(uploadConcurrencyInput, numParts)
          );
          const results = new Array(numParts);
          async function uploadPartWithRetry(partNumber, start, end) {
            for (let attempt = 1; attempt <= retryAttempts; attempt++) {
              try {
                const partStream = rawFs.createReadStream(tempArchive, {
                  start,
                  end: end - 1,
                });
                const res = await s3Client.send(
                  new UploadPartCommand({
                    Bucket: bucketName,
                    Key: fullPath,
                    PartNumber: partNumber,
                    UploadId: uploadId,
                    Body: partStream,
                  })
                );
                return { ETag: res.ETag, PartNumber: partNumber };
              } catch (err) {
                if (attempt >= retryAttempts) {
                  core.error(
                    `Upload part #${partNumber} failed after ${attempt} attempts: ${err.message}`
                  );
                  throw err;
                }
                const backoff =
                  retryBaseDelayMs * Math.pow(2, attempt - 1) +
                  Math.floor(Math.random() * retryBaseDelayMs);
                core.warning(
                  `Upload part #${partNumber} attempt ${attempt} failed: ${err.message}. Retrying in ${backoff}ms`
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
              const start = i * PART_SIZE;
              const end = Math.min(start + PART_SIZE, fileSize);
              const res = await uploadPartWithRetry(i + 1, start, end);
              results[i] = res;
            }
          }
          const workers = [];
          for (let w = 0; w < uploadConcurrency; w++) {
            workers.push(worker());
          }
          await Promise.all(workers);
          const uploadedParts = results.filter(Boolean).sort((a, b) => a.PartNumber - b.PartNumber);
          await s3Client.send(
            new CompleteMultipartUploadCommand({
              Bucket: bucketName,
              Key: fullPath,
              UploadId: uploadId,
              MultipartUpload: {
                Parts: uploadedParts,
              },
            })
          );
        } catch (err) {
          // Abort on error
          await s3Client.send(
            new AbortMultipartUploadCommand({
              Bucket: bucketName,
              Key: fullPath,
              UploadId: uploadId,
            })
          );
          throw err;
        }
      }
    }
    // Clean up temps
    await fs.unlink(tempTar).catch(() => {});
    await fs.unlink(tempArchive).catch(() => {});
    console.log(`Saved cache from ${localPath} to ${fullPath}`);
  } catch (error) {
    core.setFailed(error.message);
  }
}
run();
