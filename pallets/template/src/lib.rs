#![cfg_attr(not(feature = "std"), no_std)]

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
		traits::{Currency,ExistenceRequirement, tokens::fungibles::{Create, Transfer} },
		PalletId, Twox128,
	};
	use frame_system::Origin;


	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, Saturating,
			StaticLookup, Zero,
		},
		ArithmeticError, TokenError,
	};
	use sp_std::vec::Vec;


	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn components)]
	pub type Components<T: Config> =
		StorageMap<_, Twox64Concat, T::AssetId, Vec<T::AssetId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rates)]
	pub type Rates<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, Vec<u32>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T:Config> = StorageMap<_,Twox64Concat, T::AssetId, T::AccountId>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
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

	//impl<T: Config<I>,I:'static = ()> Pallet<T> {
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_portofio(
			origin: OriginFor<T>,
			#[pallet::compact] id: T::AssetId,
			admin: <T::Lookup as StaticLookup>::Source,
			components: Vec<T::AssetId>,
			exchange_rates: Vec<u32>,
			mint_amount: T::Balance,
		) -> DispatchResult {
			ensure!(components.len() == exchange_rates.len(), Error::<T>::NotEquel);
			for cid in &components {
				//ensure!(pallet_assets::Pallet::Asset::<T>::contains_key(cid),
				// Error::<T>::NotExistId);
			}
			pallet_assets::Pallet::<T>::force_create(origin.clone(), id, admin, true,T::Balance::from(1u32))?;
			pallet_assets::Pallet::<T>::mint(
				origin.clone(),
				id,
				<T::Lookup as StaticLookup>::unlookup(Self::account_id()),
				mint_amount,
			)?;

			let owner = ensure_signed(origin)?;
			Components::<T>::insert(id, components);
			Rates::<T>::insert(id, exchange_rates);
			Owners::<T>::insert(id,owner);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn create_contract(
			origin: OriginFor<T>,
			#[pallet::compact] id: T::AssetId,
			mint_amount: T::Balance,
		) -> DispatchResult {
			let owner = ensure_signed(origin.clone())?;
			pallet_assets::Pallet::<T>::force_create(origin.clone(), id,<T::Lookup as StaticLookup>::unlookup(owner), true,T::Balance::from(1u32))?;
			pallet_assets::Pallet::<T>::mint(
				origin.clone(),
				id,
				<T::Lookup as StaticLookup>::unlookup(Self::account_id()),
				mint_amount,
			)?;

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn buy(
			origin: OriginFor<T>,
			#[pallet::compact] id: T::AssetId,
			amount:u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Components::<T>::contains_key(id), Error::<T>::NotHasAsset);
			let owner = Owners::<T>::get(id).ok_or(Error::<T>::NotEquel)?;
			T::Currency::transfer(&who,&owner, amount.into(),ExistenceRequirement::KeepAlive)?;

			pallet_assets::Pallet::<T>::transfer(Origin::<T>::Signed(Self::account_id()).into(),
				id,<T::Lookup as StaticLookup>::unlookup(who),amount.into())?;
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn sell(
			origin: OriginFor<T>,
			#[pallet::compact] id: T::AssetId,
			amount: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(Components::<T>::contains_key(id), Error::<T>::NotHasAsset);
			pallet_assets::Pallet::<T>::transfer(origin.clone(),
				id,<T::Lookup as StaticLookup>::unlookup(Self::account_id()),amount.into())?;

			let ids = Components::<T>::get(id);
			let rate = Rates::<T>::get(id);
			for i in 0..ids.len() {
				let id = ids[i];
				let rate = rate[i];
				pallet_assets::Pallet::<T>::transfer(Origin::<T>::Signed(Self::account_id()).into(),
				id,<T::Lookup as StaticLookup>::unlookup(who.clone()),(amount*rate).into())?;
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
