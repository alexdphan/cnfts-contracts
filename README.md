# cNFTs
---
cNFTs, also known as Celestia NFTs, are an implementation of CosmoWasm's cw721 NFT contract using Celestia's Rollmint
---
*NFTs have many critiques as said by many*

One of the main points of current NFTs involve scalability issues. 
More specifically, the Data Availability problem.

cNFTs are an example of how Celestia' Rollmint can allow anyone to design and deploy a sovereign rollup on Celestia in minutes.

### A step towards increasing performance, allowing future scalabilty.
---
**[CosmWasm](https://docs.cosmwasm.com/docs/1.0/#:~:text=What%20is%20CosmWasm%3F,plug%20into%20the%20Cosmos%20SDK.) is a smart contracting platform built for the Cosmos ecosystem. Simply put, it's the Cosmos (Cosm) way of using WebAssembly (Wasm).**

- Using CosmWasm, the project can be used to be built on top of the Cosmos ecosystem to provide sovereignty, process transactions quickly and communicate with other blockchains in the ecosystem.


**[Rollmint](https://docs.celestia.org/developers/rollmint/) is an ABCI (Application Blockchain Interface) implementation for sovereign rollups to deploy on top of Celestia.**

- With Rollmint, this would allow Tendermint to be replaced with a drop-in replacement that communicates directly with Celestia's Data Availability layer. This enable anyone to design and deploy a sovereign rollup on Celestia in minutes.
--- 
## How to use

1. Input parameters about your NFTs. This would include the name, link to IPFS image, description, etc.

2. Press the mint button, burning tokens in return for a cNFT.

3. Enjoy! You now have a Celestia NFT :D
---
### Task List

- [x] Create CosmWasm contracts
- [ ] Modify Contracts geared towards cNFTs
- [ ] Allow txs to be processed 
- [ ] Create a frontend
