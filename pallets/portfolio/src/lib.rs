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
			fungible::Transfer as NativeTransfer,
			tokens::fungibles::{self, Create, Mutate, Transfer},
			Currency,
		},
		PalletId,
	};

	use frame_system::pallet_prelude::*;
	use pallet_dex::{DEXManager, SwapLimit};
	use sp_runtime::traits::{AccountIdConversion, One, Saturating, Zero};
	use sp_std::{vec, vec::Vec};

	// type T::AssetId = <<T as Config>::Currency as fungibles::Inspect<
	// 	<T as frame_system::Config>::AccountId,
	// >>::AssetId;
	// type T::Balance = <<T as Config>::Currency as fungibles::Inspect<
	// 	<T as frame_system::Config>::AccountId,
	// >>::Balance;
	// type NativeBalanceOf<T> = <<T as Config>::NativeCurrency as fungible::Inspect<
	// 	<T as frame_system::Config>::AccountId,
	// >>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		// type Currency: fungibles::Create<Self::AccountId>
		// 	+ fungibles::Mutate<Self::AccountId>
		// 	+ fungibles::Transfer<Self::AccountId>;

		type NativeCurrency: NativeTransfer<Self::AccountId>;

		type DexManager: DEXManager<Self::AccountId, Self::AssetId, Self::Balance>;
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
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn swap_paths)]
	pub type SwapPaths<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, Vec<Vec<T::AssetId>>>;

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
		NotHasPath,
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn create_portofio(
			origin: OriginFor<T>,
			id: T::AssetId,
			components: Vec<T::AssetId>,
			exchange_rates: Vec<u32>,
			mint_amount: T::Balance,
		) -> DispatchResult {
			ensure!(components.len() == exchange_rates.len(), Error::<T>::NotEquel);
			for cid in &components {
				//ensure!(pallet_assets::Pallet::Asset::<T>::contains_key(cid),
				// Error::<T>::NotExistId);
			}
			let admin = ensure_signed(origin)?;

			<pallet_assets::Pallet<T> as Create<T::AccountId>>::create(
				id,
				admin.clone(),
				true,
				T::Balance::one(),
			)?;

			pallet_assets::Pallet::<T>::mint_into(id, &Self::account_id(), mint_amount)?;

			for i in 0..components.len() {
				pallet_assets::Pallet::<T>::mint_into(
					components[i],
					&Self::account_id(),
					mint_amount.saturating_mul(exchange_rates[i].into()),
				)?;
			}

			Components::<T>::insert(id, components);
			Rates::<T>::insert(id, exchange_rates);
			Owners::<T>::insert(id, admin);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn set_swap_path(
			origin: OriginFor<T>,
			port_id: T::AssetId,
			paths: Vec<Vec<T::AssetId>>,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			SwapPaths::<T>::insert(port_id, paths);
			Ok(())
		}

		// #[pallet::weight(10_000)]
		// pub fn create_contract(
		// 	origin: OriginFor<T>,
		// 	id: T::AssetId,
		// 	mint_amount: T::Balance,
		// ) -> DispatchResult {
		// 	let owner = ensure_signed(origin.clone())?;
		// 	<pallet_assets::Pallet<T> as Create<T::AccountId>>::create(
		// 		id, owner,
		// 		true,
		// 		T::Balance::one(),
		// 	)?;
		// 	pallet_assets::Pallet::<T>::mint_into(
		// 		id,
		// 		&Self::account_id(),
		// 		mint_amount)?;

		// 	Ok(())
		// }

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn buy(origin: OriginFor<T>, id: T::AssetId, amount: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Components::<T>::contains_key(id), Error::<T>::NotHasAsset);
			let owner = Owners::<T>::get(id).ok_or(Error::<T>::NotEquel)?;
			<T::NativeCurrency as NativeTransfer<T::AccountId>>::transfer(
				&who,
				&owner,
				amount.into(),
				true,
			)?;

			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				id,
				&Self::account_id(),
				&who,
				amount.into(),
				false,
			)?;
			Ok(())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn sell(
			origin: OriginFor<T>,
			port_id: T::AssetId,
			dst_id: T::AssetId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(Components::<T>::contains_key(port_id), Error::<T>::NotHasAsset);
			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				port_id,
				&who,
				&Self::account_id(),
				amount.into(),
				false,
			)?;

			let ids = Components::<T>::get(port_id);
			let idx = ids.iter().position(|id| *id == dst_id).ok_or(Error::<T>::NotHasAsset)?;

			let rate = Rates::<T>::get(port_id);
			for i in 0..ids.len() {
				if i == idx {
					continue
				}
				let exchange_amount = amount.saturating_mul(rate[i].into());

				let best_path = {
					let saved_path = Self::swap_paths(port_id).unwrap_or_default();
					if !saved_path.is_empty() {
						T::DexManager::get_best_price_swap_path(
							ids[i],
							dst_id,
							SwapLimit::ExactSupply(exchange_amount, T::Balance::zero()),
							saved_path,
						)
						.unwrap_or_default()
					} else {
						vec![ids[i], dst_id]
					}
				};

				ensure!(!best_path.is_empty(), Error::<T>::NotHasPath);

				let (_, acture_out) = T::DexManager::swap_with_specific_path(
					&Self::account_id(),
					&best_path,
					SwapLimit::ExactSupply(exchange_amount, T::Balance::zero()),
				)?;

				<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
					dst_id,
					&Self::account_id(),
					&who,
					acture_out,
					false,
				)?;
			}
			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				dst_id,
				&Self::account_id(),
				&who,
				amount.saturating_mul(rate[idx].into()),
				false,
			)?;
			Ok(())
		}
	}
}
