import { expect } from 'chai'
import hre from 'hardhat'

describe('Eip7702SimpleAccount native deposits', () => {
  it('accepts multiple native deposits after delegation is installed', async () => {
    const [sendWallet, recipientWallet] = await hre.viem.getWalletClients()
    const sender = sendWallet
    const recipient = recipientWallet.account.address

    const contract = await hre.viem.deployContract('Eip7702SimpleAccount')
    const publicClient = await hre.viem.getPublicClient()
    const testClient = await hre.viem.getTestClient()

    // Baseline: first deposit succeeds while the account still has empty code
    const initialBalance = await publicClient.getBalance({ address: recipient })
    const firstTx = await sender.sendTransaction({
      account: sender.account,
      to: recipient,
      value: 1n
    })
    const firstReceipt = await publicClient.waitForTransactionReceipt({ hash: firstTx })
    expect(firstReceipt.status).to.equal('success')
    const firstBalance = await publicClient.getBalance({ address: recipient })
    expect(firstBalance - initialBalance).to.equal(1n)

    // After delegation (0xef0100 || delegate), subsequent deposits should still succeed.
    const delegationCode = (`0xef0100${contract.address.slice(2)}`) as `0x${string}`
    await testClient.setCode({
      address: recipient,
      bytecode: delegationCode
    })

    const secondTx = await sender.sendTransaction({
      account: sender.account,
      to: recipient,
      value: 2n
    })
    const secondReceipt = await publicClient.waitForTransactionReceipt({ hash: secondTx })
    expect(secondReceipt.status).to.equal('success')
    const secondBalance = await publicClient.getBalance({ address: recipient })
    expect(secondBalance - firstBalance).to.equal(2n)

    const thirdTx = await sender.sendTransaction({
      account: sender.account,
      to: recipient,
      value: 5n
    })
    const thirdReceipt = await publicClient.waitForTransactionReceipt({ hash: thirdTx })
    expect(thirdReceipt.status).to.equal('success')
    const finalBalance = await publicClient.getBalance({ address: recipient })
    expect(finalBalance - secondBalance).to.equal(5n)
  })
})
