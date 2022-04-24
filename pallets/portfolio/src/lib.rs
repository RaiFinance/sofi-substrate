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
	#[pallet::getter(fn port_total_balances)]
	pub type PortTotalBalances<T: Config> = StorageValue<_, Vec<T::Balance>, ValueQuery>;

	#[pallet::storage]
	pub type SwapPaths<T: Config> = StorageValue<_, Vec<Vec<T::AssetId>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		PortofioCreated(T::AssetId),
		PortofioBuy(T::Balance),
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
		NotOwner,
		NotChange,
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
		) -> DispatchResult {
			let len = components.len();
			ensure!(len == exchange_rates.len(), Error::<T>::NotEquel);
			for cid in &components {
				//ensure!(pallet_assets::Pallet::Asset::<T>::contains_key(cid),
				// Error::<T>::NotExistId);
			}
			let admin = ensure_signed(origin)?;

			let sum: u32 = exchange_rates.iter().sum();
			ensure!(sum == 100, Error::<T>::NotEquel);

			let exchange_rates: Vec<_> =
				exchange_rates.into_iter().map(|i| Perbill::from_percent(i)).collect();

			let pallet_account = Self::account_id();
			<pallet_assets::Pallet<T> as Create<T::AccountId>>::create(
				port_id,
				pallet_account,
				true,
				T::Balance::one(),
			)?;

			//pallet_assets::Pallet::<T>::mint_into(port_id, &Self::account_id(), 1u32.into())?;

			Components::<T>::insert(port_id, components);
			Rates::<T>::insert(port_id, exchange_rates);
			Owners::<T>::insert(port_id, admin);
			let zero = T::Balance::zero();
			PortTotalBalances::<T>::put(vec![zero; len]);
			Self::deposit_event(Event::PortofioCreated(port_id));
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
			//let owner = Owners::<T>::get(port_id).ok_or(Error::<T>::NotEquel)?;

			let ids = Components::<T>::get(port_id);
			let rates = Rates::<T>::get(port_id);
			let amount: T::Balance = UniqueSaturatedFrom::unique_saturated_from(amount);
			Self::deposit_event(Event::PortofioBuy(amount));

			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				0u32.into(),
				&who,
				&Self::account_id(),
				amount.clone(),
				false,
			)?;

			pallet_assets::Pallet::<T>::mint_into(port_id, &who, amount.clone())?;

			let balances = Self::do_buy(amount, ids, rates)?;
			let mut saved_balances = PortTotalBalances::<T>::get();

			for i in 0..balances.len() {
				saved_balances[i] += balances[i];
			}
			PortTotalBalances::<T>::put(saved_balances);
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
			let amount: T::Balance = UniqueSaturatedFrom::unique_saturated_from(amount);
			let whole =
				<pallet_assets::Pallet<T> as Inspect<T::AccountId>>::total_issuance(port_id);
			ensure!(whole >= amount, Error::<T>::NotHasEnoughAsset);

			pallet_assets::Pallet::<T>::burn_from(port_id, &who, amount)?;

			let perbill_rate = Perbill::from_rational(amount, whole);

			let mut saved_balances = PortTotalBalances::<T>::get();
			//let rate = Rates::<T>::get(port_id);
			let total = Self::do_sell(&mut saved_balances, ids, perbill_rate, dst_id)?;
			<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
				dst_id,
				&Self::account_id(),
				&who,
				total,
				false,
			)?;
			PortTotalBalances::<T>::put(saved_balances);
			Ok(())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn change_rate(
			origin: OriginFor<T>,
			port_id: T::AssetId,
			new_rates: Vec<u32>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			ensure!(Owners::<T>::get(port_id) == Some(owner), Error::<T>::NotOwner);

			let sum: u32 = new_rates.iter().sum();
			ensure!(sum == 100, Error::<T>::NotEquel);

			let new_rates: Vec<_> =
				new_rates.into_iter().map(|i| Perbill::from_percent(i)).collect();
			let old_rates = Rates::<T>::get(port_id);
			ensure!(new_rates.len() == old_rates.len(), Error::<T>::NotEquel);
			ensure!(new_rates != old_rates, Error::<T>::NotChange);

			let mut saved_balances = PortTotalBalances::<T>::get();
			let ids = Components::<T>::get(port_id);
			let total = Self::do_sell(
				&mut saved_balances,
				ids.clone(),
				Perbill::from_percent(100),
				0u32.into(),
			)?;

			let balances = Self::do_buy(total, ids, new_rates)?;
			PortTotalBalances::<T>::put(balances);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn do_buy(
			amount: T::Balance,
			ids: Vec<T::AssetId>,
			rates: Vec<Perbill>,
		) -> sp_std::result::Result<Vec<T::Balance>, DispatchError> {
			let mut balances = Vec::new();
			for i in 0..ids.len() {
				let balance = rates[i] * amount;
				if ids[i] == 0u32.into() {
					balances.push(balance);
					continue
				}
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

			Ok(balances)
		}

		#[transactional]
		pub fn do_sell(
			saved_balances: &mut Vec<T::Balance>,
			ids: Vec<T::AssetId>,
			perbill_rate: Perbill,
			dst_id: T::AssetId,
		) -> sp_std::result::Result<T::Balance, DispatchError> {
			let mut total = T::Balance::zero();
			for i in 0..ids.len() {
				let exchange_amount = perbill_rate * saved_balances[i];
				saved_balances[i] -= exchange_amount;
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
			Ok(total)
		}
	}
}
