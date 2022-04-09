+ [ ] Build NFT pallet
  + [ ] Storage 
    ```rust
    #[pallet::storage]
    pub(super) type CollectionById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NFTCollection<T>>;

    #[pallet::storage]
    pub(super) type TokenById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NonFungibleToken<T>>;

    #[pallet::storage]
    pub(super) type TokenSale<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Sale<T>>;
    ```

    ```rust 
    pub struct NFTCollection {
      name,
      description,
      creator
    }

    // Metadata trong near la thong tin bo sung ma creator them vao nft
    pub struct Token {
      pub title: Option<String>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
      pub description: Option<String>, // free-form description
      pub media: Option<String>, // URL to associated media, preferably to decentralized, content-addressed storage
      pub owner: Option<AccountId>,
      pub royalty: Vec<(T::AccountId, u32)>, 
      pub co_owner: Option<AccountId>,
      pub collection_id
    }
    ```
  + [ ] Function:
    + [ ] Mint // (done)
    + [ ] Edit // (done)
    + [ ] Transfer // a marketplace just buy, not transfer 
    + [ ] transferFrom // a marketplace just buy, not transfer 
    + [ ] Burn // (done)
    + [ ] balanceOf 
    + [ ] ownerOf 
    + [ ] approve // no need cus pallet have no id to get approve for. So we just our own marketplace (1)
    + [ ] getApproved // (1)
    + [ ] setApprovalForAll // (1)
    + [ ] isApprovedForAll // (1)
    + [ ] _exists
    + [ ] _isApprovedOrOwner // (1)
+ [ ] Marketplace pallet
  + [ ] Storage
  ```rust
  pub struct Sale{
    pub owner: Option<T::AccountId>,
		pub price: Option<BalanceOf<T>>,
		pub in_installment: Option<bool>
  }
  ```
  + [ ] Function
    + [ ] create sale // (done)
    + [ ] sale update(set price) // (done)
    + [ ] purchase(buy) // (done)
    + [ ] pay installments
    + [ ] withdraw sales
    + [ ] withdraw cash
    + [ ] pool lending
  

## Flow notes
+ Backend Hash -> send hash to contract -> update metadata on contract -> emit event -> get uri from event -> update storage. 




## Reference
+ https://dev.to/edge-and-node/uploading-files-to-ipfs-from-a-web-application-50a