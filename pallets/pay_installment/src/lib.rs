#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		sp_runtime::traits::{Scale},
		pallet_prelude::*,
		traits::{tokens::ExistenceRequirement, Currency, Time, ReservableCurrency},
		transactional
	};
	use frame_system::pallet_prelude::*;
	use scale_info::{TypeInfo, StaticTypeInfo};
	// use crate::weights::WeightInfo;
	use frame_support::sp_runtime::traits::Saturating;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type BalanceOf<T> =
		<<T as Config>::CurrencyOrder as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PayInstallmentOrder<Account, Balance, Time> {
		pub creator: Account,
		pub pay_at: Time,
        pub pay_per_period: Balance,
        pub periods_left: u8
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type CurrencyOrder: ReservableCurrency<Self::AccountId> + Currency<Self::AccountId>;

		// type WeightInfo: WeightInfo;

		type Moment: Parameter
			+ Default
			+ Scale<Self::BlockNumber, Output = Self::Moment>
			+ Copy
			+ MaxEncodedLen
			+ StaticTypeInfo
			+ MaybeSerializeDeserialize
			+ Send;

		type Timestamp: Time<Moment = Self::Moment>;
	}

	// Errors
	#[pallet::error]
	pub enum Error<T> {
		NoOrder,
		FromOneToSixMonths,
		PriceNotMatch,
		NotSelling
	}

	// Events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Paid { nft_id: [u8; 16], periods_left: u8 },
	}

	#[pallet::storage]
	#[pallet::getter(fn collection_by_id)]
	pub(super) type OrderByTokenId<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], PayInstallmentOrder<T::AccountId, BalanceOf<T>, T::Moment>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[transactional]
		pub fn pay_installment(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			periods: u8,
			pay_per_period: BalanceOf<T>
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			ensure!(periods > 0, Error::<T>::FromOneToSixMonths);
			ensure!(periods < 7, Error::<T>::FromOneToSixMonths);

			let nft_on_sale = pallet_nft::Pallet::<T>::token_sale(nft_id.clone()).ok_or(Error::<T>::NotSelling).unwrap();
			let price = Self::balance_to_u8(nft_on_sale.price.unwrap() as T::CurrencyOrder);
			let price_to_pay = (price/Self::u8_to_balance(periods)).saturating_mul(Self::u8_to_balance(100u8)).ceil().saturating_div(Self::u8_to_balance(100u8));
			ensure!(pay_per_period == price_to_pay, Error::<T>::PriceNotMatch);

			let mut order = OrderByTokenId::<T>::get(&nft_id);
			let periods_left = periods.clone() - 1;
			match order {
				Some(order) => {
					if periods_left == 0 {
						OrderByTokenId::<T>::remove(&nft_id)
					}
					order.periods_left = periods_left;
				},
				None => {
					let order = PayInstallmentOrder::<T::AccountId, BalanceOf<T>, T::Moment> {
						creator: &sender,
						pay_at: T::Timestamp::now(),
						pay_per_period,
						periods_left
					};
					OrderByTokenId::<T>::insert(&nft_id, order);
				}
			}

			let nft = pallet_nft::Pallet::<T>::token_by_id(nft_id.clone());

			T::Currency::transfer(&sender, &nft.owner, order.pay_per_period, ExistenceRequirement::KeepAlive)?;
			// Deposit our "Created" event.
			Self::deposit_event(Event::Paid { nft_id, periods_left });

			Ok(())
		}
	}

	//** Our helper functions.**//
	impl<T: Config> Pallet<T> {
		pub fn balance_to_u8(input: BalanceOf<T>) -> u8 {
			TryInto::<u8>::try_into(input).ok().unwrap()
		}

		pub fn u8_to_balance(input: u8) -> BalanceOf<T> {
			input.into()
		}
	}
}