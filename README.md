+ [ ] Build NFT pallet
  + [ ] Storage 
    ```rust
    Mapping<u128, TokenMetadata> idToMetadata; 
    ```

    ```rust 
    pub struct TokenMetadata {
      pub title: Option<String>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
      pub description: Option<String>, // free-form description
      pub media: Option<String>, // URL to associated media, preferably to decentralized, content-addressed storage
      pub media_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
      pub creator: Option<AccountId>,
      
      pub co_owner: Option<AccountId>,
    }
    ```
  + [ ] Function:
    + [ ] Mint
    + [ ] Transfer
    + [ ] transferFrom
    + [ ] Burn
    + [ ] balanceOf
    + [ ] ownerOf
    + [ ] approve
    + [ ] getApproved
    + [ ] setApprovalForAll
    + [ ] isApprovedForAll
    + [ ] _exists
    + [ ] _isApprovedOrOwner
+ [ ] Marketplace pallet
  + [ ] Storage
  ```rust
  pub struct Sale{
    pub seller: AccountId,
    pub price: Balance,
    pub token_id: u128,
  }
  ```
  + [ ] Function
    + [ ] create sale
    + [ ] purchase
    + [ ] pay installments
    + [ ] withdraw sales
    + [ ] sale update
    + [ ] withdraw cash
    + [ ] pool lending
  

## Flow notes
+ Backend Hash -> send hash to contract -> update metadata on contract -> emit event -> get uri from event -> update storage. 




## Reference
+ https://dev.to/edge-and-node/uploading-files-to-ipfs-from-a-web-application-50a