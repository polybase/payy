import hre from 'hardhat'
import { encodeFunctionData } from 'viem'

async function main(): Promise<void> {
  const rollupProxyAdminAddr = process.env.ROLLUP_PROXY_ADMIN_ADDR as `0x${string}` | undefined
  if (rollupProxyAdminAddr === undefined) throw new Error('ROLLUP_PROXY_ADMIN_ADDR is not set')

  const proxyRollupAddress = process.env.ROLLUP_CONTRACT_ADDR as `0x${string}` | undefined
  if (proxyRollupAddress === undefined) throw new Error('ROLLUP_CONTRACT_ADDR is not set')

  // // This code is based on a test upgrade to a V2 version.
  // // It was working when I tested on a sample V2 contract,
  // // so it will be useful when we want to actually add a new version.
  // const [owner] = await hre.viem.getWalletClients()

  const rollupProxy = await hre.viem.getContractAt('TransparentUpgradeableProxy', proxyRollupAddress)

  let version = await (await hre.viem.getContractAt('RollupV1', rollupProxy.address)).read.version()

  if (version === 1) {
    const rollupV2 = await hre.viem.deployContract('RollupV2', [])
    console.log(`ROLLUP_V2_CONTRACT_ADDR=${rollupV2.address}`)

    const initializeV2Data = encodeFunctionData({
      abi: [rollupV2.abi.find(x => x.type === 'function' && x.name === 'initializeV2') as any],
      // @ts-expect-error We know the ABI has this function
      name: 'initializeV2',
      args: []
    })
    console.log(`ROLLUP_V2_INITIALIZE_V2_CALLDATA=${initializeV2Data}`)
    version = 2
  }

  if (version === 2) {
    const rollupV3 = await hre.viem.deployContract('RollupV3', [])
    console.log(`ROLLUP_V3_CONTRACT_ADDR=${rollupV3.address}`)

    const initializeV3Data = encodeFunctionData({
      abi: [rollupV3.abi.find(x => x.type === 'function' && x.name === 'initializeV3') as any],
      // @ts-expect-error We know the ABI has this function
      name: 'initializeV3',
      args: []
    })
    console.log(`ROLLUP_V3_INITIALIZE_V3_CALLDATA=${initializeV3Data}`)
    version = 3
  }

  if (version === 3) {
    const rollupV4 = await hre.viem.deployContract('RollupV4', [])
    console.log(`ROLLUP_V4_CONTRACT_ADDR=${rollupV4.address}`)

    const initializeV4Data = encodeFunctionData({
      abi: [rollupV4.abi.find(x => x.type === 'function' && x.name === 'initializeV4') as any],
      // @ts-expect-error We know the ABI has this function
      name: 'initializeV4',
      args: []
    })
    console.log(`ROLLUP_V4_INITIALIZE_V4_CALLDATA=${initializeV4Data}`)
    version = 4
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
