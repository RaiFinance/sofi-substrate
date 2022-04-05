#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{self, Transfer as NativeTransfer},
			tokens::fungibles::{self, Create, Mutate, Transfer},
			Currency,
		},
		PalletId,
	};

	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{AccountIdConversion, One, StaticLookup};
	use sp_std::vec::Vec;

	type AssetIdOf<T> = <<T as Config>::Currency as fungibles::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::AssetId;
	type BalanceOf<T> = <<T as Config>::Currency as fungibles::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	type NativeBalanceOf<T> = <<T as Config>::NativeCurrency as fungible::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type Currency: fungibles::Create<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId>
			+ fungibles::Transfer<Self::AccountId>;

		type NativeCurrency: NativeTransfer<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn components)]
	pub type Components<T: Config> =
		StorageMap<_, Twox64Concat, AssetIdOf<T>, Vec<AssetIdOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rates)]
	pub type Rates<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, Vec<u32>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, AssetIdOf<T>, T::AccountId>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		NotEquel,
		NotExistId,
		NotHasAsset,
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_portofio(
			origin: OriginFor<T>,
			id: AssetIdOf<T>,
			components: Vec<AssetIdOf<T>>,
			exchange_rates: Vec<u32>,
			mint_amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(components.len() == exchange_rates.len(), Error::<T>::NotEquel);
			for cid in &components {
				//ensure!(pallet_assets::Pallet::Asset::<T>::contains_key(cid),
				// Error::<T>::NotExistId);
			}
			let admin = ensure_signed(origin)?;
			T::Currency::create(id, admin.clone(), true, BalanceOf::<T>::one())?;
			T::Currency::mint_into(id, &Self::account_id(), mint_amount)?;

			Components::<T>::insert(id, components);
			Rates::<T>::insert(id, exchange_rates);
			Owners::<T>::insert(id, admin);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn create_contract(
			origin: OriginFor<T>,
			id: AssetIdOf<T>,
			mint_amount: BalanceOf<T>,
		) -> DispatchResult {
			let owner = ensure_signed(origin.clone())?;
			T::Currency::create(id, owner, true, BalanceOf::<T>::one())?;
			T::Currency::mint_into(id, &Self::account_id(), mint_amount)?;

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn buy(origin: OriginFor<T>, id: AssetIdOf<T>, amount: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Components::<T>::contains_key(id), Error::<T>::NotHasAsset);
			let owner = Owners::<T>::get(id).ok_or(Error::<T>::NotEquel)?;
			T::NativeCurrency::transfer(&who, &owner, amount.into(), true)?;

			T::Currency::transfer(id, &Self::account_id(), &who, amount.into(), true)?;
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn sell(origin: OriginFor<T>, id: AssetIdOf<T>, amount: u32) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(Components::<T>::contains_key(id), Error::<T>::NotHasAsset);
			T::Currency::transfer(id, &who, &Self::account_id(), amount.into(), true)?;

			let ids = Components::<T>::get(id);
			let rate = Rates::<T>::get(id);
			for i in 0..ids.len() {
				let id = ids[i];
				let rate = rate[i];
				T::Currency::transfer(id, &Self::account_id(), &who, (amount * rate).into(), true)?;
			}
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000)]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
