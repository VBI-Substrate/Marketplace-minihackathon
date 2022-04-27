#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_runtime::DispatchResult;

#[frame_support::pallet]
pub mod pallet {
	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{ensure, pallet_prelude::{*, ValueQuery}, traits::{Get, fungibles::metadata}, Parameter, Twox64Concat, BoundedVec};
	use frame_system::{pallet_prelude::{*, OriginFor}, Origin};

	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, One, Zero},
		ArithmeticError, DispatchError, DispatchResult, RuntimeDebug,
	};

	use sp_std::vec::Vec;
	// type BalanceOf<T> =
	// <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum NftStatus {
		Normal,
		Selling,
		PayingInstalment,
	}
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum SellType {
		Normal, 
		InstalmentSell,
		Auction
	}
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum InstalmentInterestRatePerDay{
		Low,
		Medium,
		High
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct CollectionInfo<AccountId> {
		/// Token owner
		name: Vec<u8>,
		symbol: Vec<u8>,
		total_supply: u64,
		issuer: AccountId,
	}

	#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct NftInfo <AccountId, CollectionId> {
		collection_id: CollectionId,
		title: Vec<u8>,
		description: Vec<u8>,
		metadata: Vec<u8>,
		issuer: AccountId,
		pub owner: AccountId,
		pub nft_status: NftStatus,
		is_locked: bool,
		is_hidden: bool,
	}
	#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo)]
	pub struct NftSellOrder<AccountId, NftId, Time> {
		nft_id: NftId,
		price: u128,
		pub expired: Time,
		sell_type: SellType,
		creator: AccountId,
		instalment_account: Option<AccountId>,
		instalment_period: Option<Time>,
		instalment_interest_rate_per_day: Option<InstalmentInterestRatePerDay>,
		start_date: Time,
		last_paid_date: Option<Time>,
		paid: u128,
		next_pay_amount: u128,
	}
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type NftId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy  + MaxEncodedLen;

		type CollectionId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy  + MaxEncodedLen;

		type SellId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy  + MaxEncodedLen;

	}

	// pub type CollectionInfoOf<T> = CollectionInfo<<T as frame_system::Config>::AccountId> ;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]

	pub struct Pallet<T>(_);
	/*
	
		Collection storage 
	*/
	#[pallet::storage]
	#[pallet::getter(fn collection_info_of)]
	pub type Collections<T: Config> = StorageMap<_, Twox64Concat, T::CollectionId, CollectionInfo<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn collection_count)]
	pub type CollectionCount<T: Config> = StorageValue<_, T::CollectionId, ValueQuery>;

	/*
		NFT storage
	*/
	#[pallet::storage]
	#[pallet::getter(fn nft_info_of)]
	pub type Nfts<T: Config> = StorageMap <_, Twox64Concat, T::NftId, NftInfo<T::AccountId, T::CollectionId>>;

	#[pallet::storage]
	#[pallet::getter(fn nfts_count)]
	pub type NftsCount<T: Config> = StorageValue<_, T::NftId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nfts_by_owner)]
	pub type NftsByOwner<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<T::NftId>>;

	#[pallet::storage]
	#[pallet::getter(fn owner_of)]
	pub type OwnerOf<T: Config> = StorageMap<_, Twox64Concat, T::NftId, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn nfts_by_collection)]
	pub type NftsByCollection<T: Config> = StorageMap<_, Twox64Concat, T::CollectionId, Vec<T::NftId>>;

	#[pallet::storage]
	#[pallet::getter( fn sell_of_nft)]
	pub type SellOfNft<T: Config> = StorageMap<_, Twox64Concat, T::NftId, T::SellId>;

	/*
		NFT sell & buy storage
	*/

	pub type SellingInfoOf<T> = NftSellOrder< <T as frame_system::Config>::AccountId, <T as Config>::NftId, <T as frame_system::Config>::BlockNumber > ;
	#[pallet::storage]
	#[pallet::getter( fn selling_info_of)]
	pub type SellingInfo<T: Config> = StorageMap<_, Twox64Concat, T::SellId, SellingInfoOf<T>>;

	#[pallet::storage]
	#[pallet::getter( fn selling_count)]
	pub type SellingCount<T: Config> = StorageValue<_, T::SellId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter( fn selling_by_owner)]
	pub type SellingByOwner<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<T::SellId>>;



	
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		MintNft(T::AccountId, T::NftId),
		CreateSale(T::AccountId, T::SellId),
		TransferFrom(T::AccountId, T::AccountId, T::NftId),
		BuyNft(T::AccountId, T::NftId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		CollectionNotExist,
		NftIsNotExist,
		UnknownOffchainMux,
	}
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {

			log::info!("Hello from pallet-ocw.");

			// worker will be called after a day
			const TX_TYPES: u32 = 144000;
			let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) % TX_TYPES);
			let result = match modu {
				0 => Self::remove_outdate_sales(block_number),
				_ => (),
			};
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn create_collection(origin: OriginFor<T>, name: Vec<u8>, symbol: Vec<u8>, total_supply: u64) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let collection_info = CollectionInfo {
				name: name,
				symbol: symbol,
				total_supply: total_supply,
				issuer: caller,
			};
			// Collections::<T>::insert()
			let collection_id = CollectionCount::<T>::try_mutate(| id | -> Result<T::CollectionId, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T>::StorageOverflow)?;
				Ok(current_id)
			})?;
			Collections::<T>::insert(collection_id, collection_info);
			Ok(())
		}
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn mint_nft(origin: OriginFor<T>, collection_id: T::CollectionId, title: Vec<u8>, description: Vec<u8>, info: Vec<u8>, metadata: Vec<u8>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let nft_id = NftsCount::<T>::try_mutate(| id | -> Result<T::NftId, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T>::StorageOverflow)?;
				Ok(current_id)
			})?;

			let nft_info = NftInfo {
				collection_id: collection_id,
				title: title,
				description: description,
				metadata: metadata,
				issuer: caller.clone(),
				owner: caller.clone(),
				nft_status: NftStatus::Normal,
				is_locked: false,
				is_hidden: false,
			};

			ensure!(Self::check_collection(&collection_id), "Collection doesn't not exists");
			// insert owner 
			OwnerOf::<T>::insert(&nft_id, &caller);
			// insert NFT
			Nfts::<T>::insert(&nft_id, nft_info);

			// insert nft by owner
			let mut list_nft_of_caller = NftsByOwner::<T>::get(&caller).unwrap_or_default();
			let _ = list_nft_of_caller.push(nft_id.clone());
			NftsByOwner::<T>::insert(&caller, list_nft_of_caller);

			// insert nft by collection
			let mut list_nft_of_collection = NftsByCollection::<T>::get(&collection_id).unwrap_or_default();
			let _ = list_nft_of_collection.push(nft_id.clone());
			NftsByCollection::<T>::insert(&collection_id, list_nft_of_collection);

			Self::deposit_event(Event::MintNft(caller,  nft_id));

			Ok(())
		}
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn create_sale(origin: OriginFor<T>, nft_id: T::NftId, price: u128, duration: T::BlockNumber, sell_type: u8, instalment_period: T::BlockNumber) -> DispatchResult {
			
			let caller = ensure_signed(origin)?;
			ensure!(OwnerOf::<T>::get(&nft_id).unwrap() == caller, "Permission Deny");
			ensure!(Nfts::<T>::contains_key(&nft_id), "NFT is not exist");
			log::info!("-----------------: {:?}", Nfts::<T>::get(&nft_id).unwrap().nft_status == NftStatus::Normal);
			ensure!(Nfts::<T>::get(&nft_id).unwrap().nft_status == NftStatus::Normal, "Nft is Selling");
			let mut nft_info = Nfts::<T>::get(&nft_id).unwrap();

			let nft_status = match sell_type {
				// auction
				1 => {
					nft_info.nft_status = NftStatus::Selling;
					SellType::Auction
				},
				_ => {
					nft_info.nft_status = NftStatus::PayingInstalment;
					SellType::InstalmentSell
				}
			};
			Nfts::<T>::insert(&nft_id, nft_info.clone());
			let instalment_account = match &nft_info.nft_status {
				NftStatus::PayingInstalment => Some(caller.clone()),
				_ => None
			};

			let now = <frame_system::Pallet<T>>::block_number();

			let expired = now + duration;

			let nft_sell_order = NftSellOrder {
				nft_id: nft_id.clone(),
				price: price,
				expired: expired,
				sell_type: nft_status,
				creator: caller.clone(),
				instalment_account: instalment_account,
				instalment_period: Some(instalment_period),
				instalment_interest_rate_per_day: None,
				start_date: now,
				last_paid_date: None,
				paid: 0,
				next_pay_amount: 0,
			};
			
			let selling_id = SellingCount::<T>::try_mutate(| id | -> Result<T::SellId, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(&One::one()).ok_or(Error::<T>::StorageOverflow)?;
				Ok(current_id)
			})?;
			SellingInfo::<T>::insert(selling_id.clone(), nft_sell_order);

			let mut selling_of_caller = SellingByOwner::<T>::get(caller.clone()).unwrap_or_default();
			let _ = selling_of_caller.push(selling_id.clone());
			SellingByOwner::<T>::insert(&caller, selling_of_caller);

			SellOfNft::<T>::insert(nft_id.clone(), selling_id.clone());

			Self::deposit_event(Event::CreateSale(caller, selling_id));
			
			Ok(())
		}
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn buyer_buy_nft(origin: OriginFor<T>, nft_id: T::NftId, pay: u128) -> DispatchResult {
			let buyer = ensure_signed(origin)?;
			
			ensure!(Nfts::<T>::contains_key(&nft_id), "NFT is not exist");
			ensure!(Nfts::<T>::get(&nft_id).unwrap().nft_status == NftStatus::Selling, "NFT is not Sell");
			let mut nft_info = Nfts::<T>::get(&nft_id).unwrap();

			let seller = nft_info.owner;
			ensure!(buyer != seller, "can not buy nft of your self");
			nft_info.nft_status = NftStatus::Normal;
			let sell_id = SellOfNft::<T>::get(&nft_id).unwrap();
			let sell_info = SellingInfo::<T>::get(&sell_id).unwrap();
			ensure!(pay >= sell_info.price, "Not enough balance");
			let _ = Self::do_transfer(&seller, &buyer, &nft_id);

			// update selling by owner 
			let mut selling_by_owner = SellingByOwner::<T>::get(&seller).unwrap_or_default();
			let index = selling_by_owner.iter().position(| x | *x == sell_id.clone()).unwrap();
			selling_by_owner.remove(index);
			SellingByOwner::<T>::insert(&seller, selling_by_owner);

			Self::deposit_event(Event::BuyNft(buyer, nft_id));

			Ok(())
		}
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn buyer_deposit_instalment(origin: OriginFor<T>, nft_id: T::NftId, deposit: u128) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			
			ensure!(Nfts::<T>::contains_key(&nft_id), "NFT is not exist");
			ensure!(Nfts::<T>::get(&nft_id).unwrap().nft_status == NftStatus::PayingInstalment, "NFT is not enable for instalment");

			let nft_info = Nfts::<T>::get(&nft_id).unwrap();
			let seller = nft_info.owner;

			
			let sell_id = SellOfNft::<T>::get(&nft_id).unwrap();
			let mut sell_info = SellingInfo::<T>::get(&sell_id).unwrap();
			let price = sell_info.price;
			if sell_info.paid + deposit >= price 
			{
				let _ = Self::do_transfer(&seller, &buyer, &nft_id);
				// update selling by owner 
				let mut selling_by_owner = SellingByOwner::<T>::get(&seller).unwrap_or_default();
				let index = selling_by_owner.iter().position(| x | *x == sell_id.clone()).unwrap();
				selling_by_owner.remove(index);
				SellingByOwner::<T>::insert(&seller, selling_by_owner);
	
				Self::deposit_event(Event::BuyNft(buyer, nft_id));
			}
			else
			{
				ensure!(sell_info.next_pay_amount <= deposit, "insufficient depoist");
				sell_info.last_paid_date = Some(<frame_system::Pallet<T>>::block_number());
				let paid = sell_info.paid + deposit;
				let remain_instalment = price - paid;
				sell_info.next_pay_amount = Self::calc_next_pay_amount(&remain_instalment, &sell_info.instalment_period.unwrap(), &sell_info.start_date);
				SellingInfo::<T>::insert(&sell_id, &sell_info);
			}
			Ok(())
		}


	}
}

	//** Our helper functions.**//

impl<T: Config> Pallet<T> {
	// nft_id: NftId,
	// price: u128,
	// expired: Time,
	// sell_type: SellType,
	// creator: AccountId,
	// instalment_account: Option<AccountId>,
	// instalment_period: Option<Time>,
	// instalment_interest_rate_per_day: Option<InstalmentInterestRatePerDay>,
	// start_date: Time,
	// last_paid_date: Option<Time>,
	// paid: u128,
	// next_pay_amount: u128,
	pub fn remove_outdate_sales(block_number: T::BlockNumber) {
		for (sell_id, sell_info) in SellingInfo::<T>::iter() {
			if(sell_info.expired < block_number){
				SellingInfo::<T>::remove(sell_id);
			}
		}
	}

	pub fn calc_next_pay_amount(remain_instalment: &u128, instalment_period: &T::BlockNumber, start_date: &T::BlockNumber) -> u128 {
		1
	}
	pub fn check_collection(collection_id: &T::CollectionId) -> bool {
		Collections::<T>::contains_key(collection_id)
	}
	pub fn do_transfer(from: &T::AccountId, to: &T::AccountId, nft_id: &T::NftId) -> DispatchResult {
		if !Nfts::<T>::contains_key(&nft_id) {
			return Err(Error::<T>::NftIsNotExist)?;
		}
		let mut nft_info = Nfts::<T>::get(&nft_id).unwrap();
		nft_info.owner = to.clone();
		OwnerOf::<T>::insert(&nft_id, &to);
		let mut nft_by_from = NftsByOwner::<T>::get(&from).unwrap_or_default();
		let index = nft_by_from.iter().position(| x | *x == nft_id.clone()).unwrap();
		nft_by_from.remove(index);
		NftsByOwner::<T>::insert(&from, nft_by_from);

		let mut nft_by_to  = NftsByOwner::<T>::get(to.clone()).unwrap_or_default();
		nft_by_to.push(nft_id.clone());
		NftsByOwner::<T>::insert(&to, nft_by_to);
		Self::deposit_event(Event::TransferFrom(from.clone(), to.clone(), nft_id.clone()));

		Ok(())
	}
}