import hre from 'hardhat'

async function main(): Promise<void> {
  const rollupProxyAdminAddr = process.env.ROLLUP_PROXY_ADMIN_ADDR as `0x${string}` | undefined
  if (rollupProxyAdminAddr === undefined) throw new Error('ROLLUP_PROXY_ADMIN_ADDR is not set')

  const proxyRollupAddress = process.env.ROLLUP_CONTRACT_ADDR as `0x${string}` | undefined
  if (proxyRollupAddress === undefined) throw new Error('ROLLUP_CONTRACT_ADDR is not set')

  await hre.viem.getContractAt(
    '@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol:ProxyAdmin',
    rollupProxyAdminAddr,
  )

  const rollupProxy = await hre.viem.getContractAt(
    'TransparentUpgradeableProxy',
    proxyRollupAddress,
  )

  const version = await (
    await hre.viem.getContractAt(
      'contracts/rollup3/RollupV1.sol:RollupV1',
      rollupProxy.address,
    )
  ).read.version()

  console.log('Current version:', version)
  console.log('No upgrade steps required for RollupV1 deployments.')
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
