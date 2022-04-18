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
		pallet_prelude::{ValueQuery, *},
		traits::{
			fungible::Transfer as NativeTransfer,
			tokens::fungibles::{self, Create, Inspect, Mutate, Transfer},
			Currency,
		},
		transactional, PalletId,
	};

	use frame_system::pallet_prelude::*;
	use pallet_dex::{DEXManager, SwapLimit};
	use sp_runtime::{
		traits::{AccountIdConversion, One, UniqueSaturatedFrom, UniqueSaturatedInto, Zero},
		Perbill,
	};
	use sp_std::{vec, vec::Vec};

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
	pub type Rates<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, Vec<Perbill>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owners)]
	pub type Owners<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn port_user_balances)]
	pub type PortUserBalances<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AssetId,
		Twox64Concat,
		T::AccountId,
		Vec<T::Balance>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type SwapPaths<T: Config> = StorageValue<_, Vec<Vec<T::AssetId>>, ValueQuery>;

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
		NotHasEnoughAsset,
		NotEqueOne,
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
			port_id: T::AssetId,
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

			let sum: u32 = exchange_rates.iter().sum();
			ensure!(sum == 100, Error::<T>::NotEquel);

			let exchange_rates: Vec<_> =
				exchange_rates.into_iter().map(|i| Perbill::from_percent(i)).collect();

			<pallet_assets::Pallet<T> as Create<T::AccountId>>::create(
				port_id,
				admin.clone(),
				true,
				T::Balance::one(),
			)?;

			pallet_assets::Pallet::<T>::mint_into(port_id, &Self::account_id(), mint_amount)?;

			Components::<T>::insert(port_id, components);
			Rates::<T>::insert(port_id, exchange_rates);
			Owners::<T>::insert(port_id, admin);

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn set_swap_path(origin: OriginFor<T>, paths: Vec<Vec<T::AssetId>>) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			SwapPaths::<T>::put(paths);
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
		pub fn buy(origin: OriginFor<T>, port_id: T::AssetId, amount: u128) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Components::<T>::contains_key(port_id), Error::<T>::NotHasAsset);
			let owner = Owners::<T>::get(port_id).ok_or(Error::<T>::NotEquel)?;

			let ids = Components::<T>::get(port_id);
			let rates = Rates::<T>::get(port_id);

			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				0u32.into(),
				&who,
				&Self::account_id(),
				UniqueSaturatedFrom::unique_saturated_from(amount),
				false,
			)?;

			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				port_id,
				&Self::account_id(),
				&who,
				UniqueSaturatedFrom::unique_saturated_from(amount),
				false,
			)?;

			let mut balances = Vec::new();
			for i in 0..ids.len() {
				let balance = rates[i] * amount;
				let best_path = {
					let saved_path = SwapPaths::<T>::get();
					if !saved_path.is_empty() {
						T::DexManager::get_best_price_swap_path(
							0u32.into(),
							ids[i],
							SwapLimit::ExactSupply(
								UniqueSaturatedFrom::unique_saturated_from(balance),
								T::Balance::zero(),
							),
							saved_path,
						)
						.unwrap_or_default()
					} else {
						vec![0u32.into(), ids[i]]
					}
				};

				let (_, acture_out) = T::DexManager::swap_with_specific_path(
					&Self::account_id(),
					&best_path,
					SwapLimit::ExactSupply(
						UniqueSaturatedFrom::unique_saturated_from(balance),
						T::Balance::zero(),
					),
				)?;
				balances.push(acture_out);
			}
			PortUserBalances::<T>::insert(port_id, who, balances);
			Ok(())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn sell(
			origin: OriginFor<T>,
			port_id: T::AssetId,
			dst_id: T::AssetId,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			ensure!(Components::<T>::contains_key(port_id), Error::<T>::NotHasAsset);

			let ids = Components::<T>::get(port_id);
			//let idx = ids.iter().position(|id| *id == dst_id).ok_or(Error::<T>::NotHasAsset)?;
			let amount: T::Balance = UniqueSaturatedFrom::unique_saturated_from(amount);
			let whole = <pallet_assets::Pallet<T> as Inspect<T::AccountId>>::balance(port_id, &who);
			ensure!(whole >= amount, Error::<T>::NotHasEnoughAsset);

			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				port_id,
				&who,
				&Self::account_id(),
				amount,
				false,
			)?;

			let get_balance_rate = whole / amount;
			let get_balance_rate =
				UniqueSaturatedInto::<u64>::unique_saturated_into(get_balance_rate);
			let perbill_rate = Perbill::from_rational(1u64, get_balance_rate);

			let mut balances = PortUserBalances::<T>::get(port_id, who.clone());
			//let rate = Rates::<T>::get(port_id);
			let mut total = T::Balance::zero();
			for i in 0..ids.len() {
				let exchange_amount = perbill_rate * balances[i];
				balances[i] -= exchange_amount;
				if ids[i] == dst_id {
					total += exchange_amount;
					continue
				} else {
					let best_path = {
						let saved_path = SwapPaths::<T>::get();
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
					total += acture_out;
				}
			}
			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				dst_id,
				&Self::account_id(),
				&who,
				total,
				false,
			)?;
			PortUserBalances::<T>::insert(port_id, who, balances);
			Ok(())
		}
	}
}
