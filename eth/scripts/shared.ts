import hre from 'hardhat'
import { readFile } from 'fs/promises'

export async function deployBytecode(bytecode: string): Promise<`0x${string}`> {
  const normalized = bytecode.startsWith('0x') ? bytecode : `0x${bytecode}`

  const [owner] = await hre.viem.getWalletClients()
  const deployTx = await owner.deployContract({
    account: owner.account,
    bytecode: normalized,
    abi: []
  })

  const publicClient = await hre.viem.getPublicClient()
  const deployedAddr = (await publicClient.waitForTransactionReceipt({ hash: deployTx })).contractAddress

  if (deployedAddr === null || deployedAddr === undefined) throw new Error('Verifier address not found')

  return deployedAddr
}

export async function deployBin(binFile: string): Promise<`0x${string}`> {
  const bin = (await readFile(`contracts/${binFile}`)).toString().trimEnd()
  return deployBytecode(bin)
}
