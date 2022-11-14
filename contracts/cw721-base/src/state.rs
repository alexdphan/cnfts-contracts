use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use cosmwasm_std::{Addr, BlockInfo, CustomMsg, StdResult, Storage};

use cw721::{ContractInfoResponse, Cw721, Expiration};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

// This is where we set up our state, with a struct
// Contract is a "class", we are developing it as a "class" in rust (struct and impl)
// 'a (commonly used) is our lifetime specifier, we are using it to specify the lifetime of the contract. It's not specified in cw-template but it is in cw721-base. In this struct, everything is stored in lifetime 'a.
// Data structure is set up here
// Contract storage is monolithic, one storage only for the whole contract

pub struct Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    Q: CustomMsg,
    E: CustomMsg,
{
    // This is a reference to the CosmWasm storage. It's a reference because we don't want to own the storage, we just want to borrow it. We don't want to own it because we don't want to be able to change it. We just want to be able to read it.
    pub contract_info: Item<'a, ContractInfoResponse>,
    pub minter: Item<'a, Addr>,
    pub token_count: Item<'a, u64>,
    /// The Map() method: a function to each element in an iterable and returns the resulting iterable, of each iteration, to the next function.
    /// Stored as (granter, operator) giving operator full control over granter's account
    /// <'a, (&'a Addr, &'a Addr), Expiration> is the type of the map. 
    /// It's a map of a tuple of two addresses to an expiration. 
    /// The first address is the granter, the second address is the operator (our key serialized together).
    /// The expiration is the expiration of the operator's ability to control the granter's account (when it expires if it exists). 
    /// When a granter gives an operator permission to control their NFTs, there can be an expiration date (could be never).
    /// lifetime 'a, key type (&'a Addr, &'a Addr), value type Expiration
    pub operators: Map<'a, (&'a Addr, &'a Addr), Expiration>,
    /// lifetime 'a, key type &'a Addr, value type TokenInfo (data that is stored in the map as a struct, which can have the extension T). We also take an IndexList type which we imported above.
    /// We need to define the TokenIndexes are in order to use the IndexedMap() method. We also need to define the TokenInfo struct (what each token contains).
    /// Wouldn't need to add addition arguments because for additional indexes because TokenIndexes is a list of all indexes.
    pub tokens: IndexedMap<'a, &'a str, TokenInfo<T>, TokenIndexes<'a, T>>,

    pub(crate) _custom_response: PhantomData<C>,
    pub(crate) _custom_query: PhantomData<Q>,
    pub(crate) _custom_execute: PhantomData<E>,
}

// This is a signal, the implementations are in other files
impl<'a, T, C, E, Q> Cw721<T, C> for Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
}

impl<T, C, E, Q> Default for Cw721Contract<'static, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    E: CustomMsg,
    Q: CustomMsg,
{
    // We find the storage by these keys. Everything that lives in storage is found by these keys.
    fn default() -> Self {
        Self::new(
            "nft_info",
            "minter",
            "num_tokens",
            "operators",
            "tokens",
            "tokens__owner",
        )
    }
}

impl<'a, T, C, E, Q> Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    E: CustomMsg,
    Q: CustomMsg,
{
    // storage keys calls this new function which set all the keys as storage keys in the contract storage
    fn new(
        contract_key: &'a str,
        minter_key: &'a str,
        token_count_key: &'a str,
        operator_key: &'a str,
        tokens_key: &'a str,
        tokens_owner_key: &'a str,
    ) -> Self {
        // asks by address, returns all token ids owned by that address
        // look at TokenIndexes struct below
        let indexes = TokenIndexes {
            // new owner index which is a MultiIndex
            owner: MultiIndex::new(token_owner_idx, tokens_key, tokens_owner_key),
        };
        Self {
            contract_info: Item::new(contract_key),
            // looking at the minter_key (fn default) to get the Item that is in storage (struct Cw721Contract) at the key "minter" (fn default)
            minter: Item::new(minter_key), 
            token_count: Item::new(token_count_key),
            operators: Map::new(operator_key),
            tokens: IndexedMap::new(tokens_key, indexes),
            _custom_response: PhantomData,
            _custom_execute: PhantomData,
            _custom_query: PhantomData,
        }
    }

    // keeping track of number of tokens
    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    // incrementing the token count
    // takes the storage, storage at the the key "num_tokens" as an Item 
    // loads the value at that key, increments it by 1, and saves it back to the storage
    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }
}

// Stored for each token    
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo<T> {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: T,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

// Index by owner, approvals, token_uri, and extension (from TokenInfo)
// lifetime specifier 'a and type extension T
pub struct TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    // TokenIdexes is a MultiIndex with lifetime 'a (whole life of the contract)
    // and the key type is &'a Addr (address of the owner)
    // and the value type is TokenInfo<T> (the data stored in the map)
    // Look at the MultiIndex struct definition for more info
    // MultiIndex stores (namespace, index_name, idx_value, pk) -> b"pk_len".
    pub owner: MultiIndex<'a, Addr, TokenInfo<T>, String>,

    // can add more indexes here
}

impl<'a, T> IndexList<TokenInfo<T>> for TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo<T>>> + '_> {
        let v: Vec<&dyn Index<TokenInfo<T>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn token_owner_idx<T>(_pk: &[u8], d: &TokenInfo<T>) -> Addr {
    d.owner.clone()
}

// can add index functions here
