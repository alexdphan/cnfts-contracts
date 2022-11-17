use std::fmt::Debug; // added std::fmt::Debug for the derive(Debug) below
use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{Binary, CustomMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use cw2::set_contract_version;
use cw721::{ContractInfoResponse, Cw721Execute, Cw721ReceiveMsg, Expiration};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
use crate::state::{Approval, Cw721Contract, TokenInfo};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Rust doesn't have classes, use impl blocks to group methods (T:, C:, E:, Q:)
// https://doc.rust-lang.org/book/ch05-03-method-syntax.html
// 'a (lifetime), T (our extension), C (custom message/response), E (custom execute?), Q (custom query?) on the Cw721Contract
// implementing additional functionality (with methods) to our struct Cw721Contract
impl<'a, T, C, E, Q> Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone + Debug, // added Debug, our extension T needs to be serializable, deserializable, cloneable, and debuggable (has to have these traits)
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
    // Normally, you see a cfg that specifies that it is an entry point
    // Rather than the entry point being above each function (Instantiate, execute, query) which are split between the execute.rs and query.rs files, all the entry point notation is in the lib file. 
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response<C>> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let info = ContractInfoResponse {
            name: msg.name,
            symbol: msg.symbol,
        };
        self.contract_info.save(deps.storage, &info)?;
        let minter = deps.api.addr_validate(&msg.minter)?;
        self.minter.save(deps.storage, &minter)?;
        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<T, E>,
    ) -> Result<Response<C>, ContractError> {
        match msg {
            ExecuteMsg::Mint(msg) => self.mint(deps, env, info, msg),
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => self.approve(deps, env, info, spender, token_id, expires),
            ExecuteMsg::Revoke { spender, token_id } => {
                self.revoke(deps, env, info, spender, token_id)
            }
            ExecuteMsg::ApproveAll { operator, expires } => {
                self.approve_all(deps, env, info, operator, expires)
            }
            ExecuteMsg::RevokeAll { operator } => self.revoke_all(deps, env, info, operator),
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => self.transfer_nft(deps, env, info, recipient, token_id),
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => self.send_nft(deps, env, info, contract, token_id, msg),
            ExecuteMsg::Burn { token_id } => self.burn(deps, env, info, token_id),
            ExecuteMsg::Extension { msg: _ } => Ok(Response::default()),
        }
    }
}

// TODO pull this into some sort of trait extension??
impl<'a, T, C, E, Q> Cw721Contract<'a, T, C, E, Q>
where // contraints
    T: Serialize + DeserializeOwned + Clone + Debug, // T is a generic type that must implement (has to have) Serialize, DeserializeOwned, Clone, and now Debug (manually added)
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
    pub fn mint(
        &self, // self is the Cw721Contract, needs access to the rest of the contract (methods, state, etc); usually implicit
        deps: DepsMut, // storage, api, querier
        _env: Env, // block info, contract info, message info
        info: MessageInfo, // sender, funds
        msg: MintMsg<T>, // info about token we are minting, look at definition
    ) -> Result<Response<C>, ContractError> {
        // check if the account that's minting is authorized to mint
        // declaring a variable and load it up from storage (need access to storage)
        // added .load(store: deps.storage)
        // ? is a shortcut for returning an error if there is one
        let minter = self.minter.load(deps.storage)?;

        // doing it this way does not load the value from storage, it just creates a variable that is a reference to the storage
        // let minter: item<Addr> = self.minter; 
      
        // if the sender of the mint msg is not authorized minter (initially set when the contract is spun up) in contract storage (state.rs), return an error
        if info.sender != minter {
            return Err(ContractError::Unauthorized {});
        }

        // create the token
        // makes a TokenInfo struct that we will save to storage
        let token = TokenInfo {
            owner: deps.api.addr_validate(&msg.owner)?,
            approvals: vec![],
            token_uri: msg.token_uri,
            extension: msg.extension,
        };
        // IndexMap is a map with additional index functionality
        // Called update function
        // pass in storage
        // pass in token_id (msg.id) we are trying to update
        // pass in old token (the token we just created) and matching it with the token_id
        // If it matches, the token already claimed and we return an error
        // If it doesn't match, it's available and send the token to the update function, saving the token to storage
        self.tokens
            .update(deps.storage, &msg.token_id, |old| match old {
                Some(_) => Err(ContractError::Claimed {}),
                None => Ok(token.clone()), // token needs to be cloned
            })?;
        
        // We increment the number of tokens in the contract (function in state.rs)
        self.increment_tokens(deps.storage)?;

        // for easy local cargo test -- --show-output (to see the output of the tests)
        println!("token_info: {:?}", token.clone());
        // ex output, these unit tests would print out these NFTs like below
        // ---- tests::use_metadata_extension stdout ----
        // token_info: TokenInfo { owner: Addr("john"), approvals: [], token_uri: Some("https://starships.example.com/Starship/Enterprise.json"), extension: Some(Metadata { image: None, image_data: None, external_url: None, description: Some("Spaceship with Warp Drive"), name: Some("Starship USS Enterprise"), attributes: None, background_color: None, animation_url: None, youtube_url: None }) }

        // Ouput data in this created response
        // More useful, for block explorers when we are on testnet
        // Also useful for frontend devs or for anyone who needs to get information from the response
        // Getting attributes from resonse using js would be response.attributes[0] for example
        Ok(Response::new()
            .add_attribute("token_info", format!("{:?}", token)) // token must implement the Debug trait
            .add_attribute("action", "mint")
            .add_attribute("minter", info.sender)
            .add_attribute("owner", msg.owner)
            .add_attribute("token_id", msg.token_id))
    }
}

impl<'a, T, C, E, Q> Cw721Execute<T, C> for Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
    type Err = ContractError;

    fn transfer_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        // calls self helper function _ so we don't duplicate fn names)
        self._transfer_nft(deps, &env, &info, &recipient, &token_id)?; 

        // could be used to get more information about the token
        // let token = self._transfer_nft(deps, &env, &info, &recipient, &token_id)?; 

        // You could make a TransferNFTMsg struct that contains info, recipient, and token_id if you wanted to

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id))
    }

    // doesn't just change the owner of the nft, it also takes a transaction
    fn send_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response<C>, ContractError> {
        // Transfer token (helper function)
        // _transfer_nft is being reused as a helper function
        // sending the nft to a contract (&contract) so we can send a Cw721ReceiveMsg
        self._transfer_nft(deps, &env, &info, &contract, &token_id)?;

        // Here, we create a Cw721ReceiveMsg that has the sender (below)
        // sender is who sent the token to the contract
        // token_id is the token that was sent
        // msg is the message that was sent with the token (Binary is a type that can be converted to a string). Could want to include addtional info (message) with the token
        // ----
        // Have to be sure that address of this contract must be known to the contract that is receiving the NFT (person receiving has to know the address of the contract being sent to him/her). This is different than the sender itself. 
        // The sender is the person sending the token to the contract, but the contract itself must be known to the person receiving the token
        // Otherwise, any contract can send an NFT to this contract and it will accept it
        // ----
        // Cw721ReceiveMsg can make itself into a binary or can make itself into a cosmos message (check definition of Cw721ReceiveMsg)
        let send = Cw721ReceiveMsg {
            sender: info.sender.to_string(),
            token_id: token_id.clone(),
            msg,
        };

        // Send message
        Ok(Response::new()
        // chaining the send message to the response
        // diff than add._submessage: used for ibc, won't fail the whole tx if the submessage fails
        // .add_message: will fail the whole tx if the send message fails
            .add_message(send.into_cosmos_msg(contract.clone())?)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", contract)
            .add_attribute("token_id", token_id))
    }

    fn approve(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, ContractError> {
        self._update_approvals(deps, &env, &info, &spender, &token_id, true, expires)?;

        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn revoke(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        self._update_approvals(deps, &env, &info, &spender, &token_id, false, None)?;

        Ok(Response::new()
            .add_attribute("action", "revoke")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn approve_all(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, ContractError> {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(ContractError::Expired {});
        }

        // set the operator for us
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.operators
            .save(deps.storage, (&info.sender, &operator_addr), &expires)?;

        Ok(Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn revoke_all(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        operator: String,
    ) -> Result<Response<C>, ContractError> {
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.operators
            .remove(deps.storage, (&info.sender, &operator_addr));

        Ok(Response::new()
            .add_attribute("action", "revoke_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn burn(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        let token = self.tokens.load(deps.storage, &token_id)?;
        self.check_can_send(deps.as_ref(), &env, &info, &token)?;

        self.tokens.remove(deps.storage, &token_id)?;
        self.decrement_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "burn")
            .add_attribute("sender", info.sender)
            .add_attribute("token_id", token_id))
    }
}

// helpers
impl<'a, T, C, E, Q> Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
    pub fn _transfer_nft(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        recipient: &str,
        token_id: &str,
    ) -> Result<TokenInfo<T>, ContractError> {
        // takes the token (mutable), loads the token from the storage by token_id
        // self.tokens would be an instance of IndexedMap, so we can use .load to get the token
        // here you don't only pass in the storage, but also the key (token_id)
        let mut token = self.tokens.load(deps.storage, token_id)?;
        // ensure we have permissions
        self.check_can_send(deps.as_ref(), env, info, &token)?;
        // set owner and remove existing approvals
        // set owner to recipient (recipient is a string)
        token.owner = deps.api.addr_validate(recipient)?;
        // clear approvals, set to empty vector
        token.approvals = vec![];
        // save the token back to the storage
        self.tokens.save(deps.storage, token_id, &token)?; 
        // respond Ok with the token (the main function called will respond with the token (with add_attribute))
        Ok(token)
    } // could have used .update instead of .load and .save

    #[allow(clippy::too_many_arguments)]
    pub fn _update_approvals(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        spender: &str,
        token_id: &str,
        // if add == false, remove. if add == true, remove then set with this expiration
        add: bool,
        expires: Option<Expiration>,
    ) -> Result<TokenInfo<T>, ContractError> {
        let mut token = self.tokens.load(deps.storage, token_id)?;
        // ensure we have permissions
        self.check_can_approve(deps.as_ref(), env, info, &token)?;

        // update the approval list (remove any for the same spender before adding)
        let spender_addr = deps.api.addr_validate(spender)?;
        token.approvals.retain(|apr| apr.spender != spender_addr);

        // only difference between approve and revoke
        if add {
            // reject expired data as invalid
            let expires = expires.unwrap_or_default();
            if expires.is_expired(&env.block) {
                return Err(ContractError::Expired {});
            }
            let approval = Approval {
                spender: spender_addr,
                expires,
            };
            token.approvals.push(approval);
        }

        self.tokens.save(deps.storage, token_id, &token)?;

        Ok(token)
    }

    /// returns true iff the sender can execute approve or reject on the contract
    pub fn check_can_approve(
        &self,
        deps: Deps,
        env: &Env,
        info: &MessageInfo,
        token: &TokenInfo<T>,
    ) -> Result<(), ContractError> {
        // owner can approve
        if token.owner == info.sender {
            return Ok(());
        }
        // operator can approve
        let op = self
            .operators
            .may_load(deps.storage, (&token.owner, &info.sender))?;
        match op {
            Some(ex) => {
                if ex.is_expired(&env.block) {
                    Err(ContractError::Unauthorized {})
                } else {
                    Ok(())
                }
            }
            None => Err(ContractError::Unauthorized {}),
        }
    }

    /// returns true if the sender can transfer ownership of the token
    pub fn check_can_send(
        &self,
        deps: Deps,
        env: &Env,
        info: &MessageInfo,
        token: &TokenInfo<T>,
    ) -> Result<(), ContractError> {
        // owner can send, if owner is info.sender then we return empty Ok result
        if token.owner == info.sender {
            return Ok(());
        }

        // any non-expired token approval can send
        // if this token has any approvals, then we check if the approvals are not expired
        // if the token has approvals and none of them are expired, then we return empty Ok result
        if token
            .approvals
            .iter()
            .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
        {
            return Ok(());
        }

        // operator can send; they have approval for all the NFTs an address has (doesn't actually own, but has the approval/ability to send/transfer)
        // if there is an operator for this token owner and info.sender, then we return the operator
        let op = self
            .operators
            .may_load(deps.storage, (&token.owner, &info.sender))?;
        match op {
            // check if the operator is expired
            // if it is expired then we return an error
            // if it is not expired, then we return empty Ok result
            Some(ex) => {
                if ex.is_expired(&env.block) {
                    Err(ContractError::Unauthorized {})
                } else {
                    Ok(())
                }
            }
            None => Err(ContractError::Unauthorized {}),
        }
    }
}

// need to deploy the contract