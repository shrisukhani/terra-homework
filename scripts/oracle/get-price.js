import { Fee, MsgExecuteContract } from "@terra-money/terra.js"
import { client, wallets } from "../library.js"

const contractAddr = "terra1lyla4th5dtx85chq5qqht77kfwywgn42jgvjgs"
// const walletAddr = wallets.bombay.key.accAddress

const QUERY_PRICE_MSG = { query_price: {} }

const resp = await client.wasm.contractQuery(contractAddr, QUERY_PRICE_MSG)

console.log(resp)
