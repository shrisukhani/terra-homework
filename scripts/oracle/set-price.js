import { Fee, MsgExecuteContract } from "@terra-money/terra.js"
import { client, wallets } from "../library.js"

const contractAddr = "terra1lyla4th5dtx85chq5qqht77kfwywgn42jgvjgs"
const walletAddr = wallets.bombay.key.accAddress

const UPDATE_PRICE_MSG = { update_price: { price: 1234 } }

const tx = await wallets.bombay.createAndSignTx({
  msgs: [
    new MsgExecuteContract(
      walletAddr, // Sender
      contractAddr, // contract addr
      UPDATE_PRICE_MSG, // msg
      {
        uluna: 100000,
      } // coins
    ),
  ],
})

const result = await client.tx.broadcast(tx)

console.log(result)
