#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::{pallet::BenchmarkHelper, Pallet as Xnft};
use frame_benchmarking::{account, benchmarks_instance_pallet, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::{Config as SystemConfig, RawOrigin as SystemOrigin};
use pallet_nfts::{CollectionConfig, CollectionConfigFor, CollectionSettings, MintSettings};
use polkadot_parachain::primitives::Sibling;
use sp_core::{Decode, Encode};
use sp_runtime::{
	traits::{AccountIdConversion, Bounded, Member, StaticLookup},
	AccountId32, BoundedVec, MultiSignature,
};
const SEED: u32 = 0;
use sp_std::prelude::*;
macro_rules! bvec {
    ($( $x:tt )*) => {
        vec![$( $x )*].try_into().unwrap()
    }
}
fn sibling_a_account() -> AccountId32 {
	Sibling::from(2000).into_account_truncating()
}

fn sibling_b_account() -> AccountId32 {
	Sibling::from(2001).into_account_truncating()
}

fn convert_account_id<T: SystemConfig>(account_id: AccountId32) -> T::AccountId
where
	T::AccountId: Member,
{
	let byte_array: [u8; 32] = account_id.into();

	T::AccountId::decode(&mut &byte_array[..]).expect("Failed to convert AccountId32 to AccountId")
}

fn convert_to_account_id32<T: SystemConfig>(account_id: T::AccountId) -> AccountId32
where
	T::AccountId: Encode,
{
	let encoded: Vec<u8> = account_id.encode();
	let mut account_id32_bytes = [0u8; 32];
	account_id32_bytes.copy_from_slice(&encoded[..32]);
	AccountId32::from(account_id32_bytes)
}

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn collection_config_with_all_settings_enabled<T: Config<I>, I: 'static>(
) -> CollectionConfigFor<T, I> {
	CollectionConfig {
		settings: CollectionSettings::all_enabled(),
		max_supply: None,
		mint_settings: MintSettings::default(),
	}
}

fn create_collection<T: Config<I>, I: 'static>() -> (T::CollectionId, T::CollectionId)
where
	T: pallet_balances::Config,
	T::AccountId: From<AccountId32>,
{
	let caller: T::AccountId = whitelisted_caller();
	let account: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let collection = <T as pallet::Config<I>>::Helper::collection(0);
	T::Currency::make_free_balance_be(&caller, DepositBalanceOf::<T, I>::max_value());
	T::Currency::make_free_balance_be(&account, DepositBalanceOf::<T, I>::max_value());
	let sibling_account: AccountIdLookupOf<T> =
		T::Lookup::unlookup(convert_to_account_id32::<T>(account.clone()).into());
	let call = collection_config_with_all_settings_enabled::<T, I>();
	let dest_collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	CollectionMap::<T, I>::insert(dest_collection_id, collection);
	CollectionOwner::<T, I>::insert(dest_collection_id, account.clone());
	pallet_nfts::Pallet::<T, I>::create(
		SystemOrigin::Signed(account.clone()).into(),
		sibling_account.clone(),
		call,
	);
	(collection, dest_collection_id)
}

fn set_collection_metadata<T: Config<I>, I: 'static>() -> () {
	let caller: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let collection = <T as pallet::Config<I>>::Helper::collection(0);
	let metadata: BoundedVec<u8, <T as pallet_nfts::Config<I>>::StringLimit> = bvec![0u8; 20];
	pallet_nfts::Pallet::<T, I>::set_collection_metadata(
		SystemOrigin::Signed(caller.clone()).into(),
		collection,
		metadata.clone(),
	);
}

fn mint_nft<T: Config<I>, I: 'static>() -> T::ItemId
where
	T: pallet_balances::Config,
	T::AccountId: From<AccountId32>,
{
	let collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	let item_id = <T as pallet::Config<I>>::Helper::item(0);
	let caller: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let mint_to_sibling: AccountIdLookupOf<T> =
		T::Lookup::unlookup(convert_account_id::<T>(sibling_a_account()).into());
	pallet_nfts::Pallet::<T, I>::mint(
		SystemOrigin::Signed(caller.clone()).into(),
		collection_id,
		item_id,
		mint_to_sibling.clone(),
		None,
	);
	item_id
}

fn set_nft_metadata<T: Config<I>, I: 'static>(
) -> BoundedVec<u8, <T as pallet_nfts::Config<I>>::StringLimit> {
	let caller: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let collection = <T as pallet::Config<I>>::Helper::collection(0);
	let item_id = <T as pallet::Config<I>>::Helper::item(0);
	let metadata: BoundedVec<u8, <T as pallet_nfts::Config<I>>::StringLimit> = bvec![0u8; 20];
	pallet_nfts::Pallet::<T, I>::set_metadata(
		SystemOrigin::Signed(caller.clone()).into(),
		collection,
		item_id,
		metadata.clone(),
	);
	metadata
}

fn dest_mint_nfts<T: Config<I>, I: 'static>() -> Vec<T::ItemId>
where
	T: pallet_balances::Config,
	T::AccountId: From<AccountId32>,
{
	let caller: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let dest_collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	let dest_item_id = vec![<T as pallet::Config<I>>::Helper::item(0)];
	let item_id = vec![mint_nft::<T,I>()];
	for (i, dest_i) in item_id.iter().zip(dest_item_id.iter()) {
		AccountIds::<T, I>::insert((dest_collection_id,dest_i),caller.clone());
		ItemIdMap::<T, I>::insert((dest_collection_id, dest_i), i);
	}
	dest_item_id
}
fn insert_metadata<T: Config<I>, I: 'static>()-> BoundedVec<u8, <T as pallet_nfts::Config<I>>::StringLimit>
where
	T: pallet_balances::Config,
	T::AccountId: From<AccountId32>, {

	let collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	let dest_collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	let dest_item_id = vec![<T as pallet::Config<I>>::Helper::item(0)];
	let item_id = vec![<T as pallet::Config<I>>::Helper::item(0)];
	let metadata = set_nft_metadata::<T,I>();
	for (i, dest_i) in item_id.iter().zip(dest_item_id.iter()) {
		ItemMetadataMap::<T, I>::insert(
			(collection_id, dest_collection_id, *i, *dest_i),
			metadata.clone(),
		);
		}
	metadata

}
fn transfer_nft<T: Config<I>, I: 'static>() {
	let collection_id = <T as pallet::Config<I>>::Helper::collection(0);
	let item_id = <T as pallet::Config<I>>::Helper::item(0);
	let caller: T::AccountId = convert_account_id::<T>(sibling_a_account());
	let owner: AccountIdLookupOf<T> =
		T::Lookup::unlookup(convert_account_id::<T>(sibling_a_account()).into());
	pallet_nfts::Pallet::<T, I>::transfer(
		SystemOrigin::Signed(caller.clone()).into(),
		collection_id,
		item_id,
		owner,
	);
}

benchmarks_instance_pallet! {
	where_clause {
		where
			T::OffchainSignature: From<MultiSignature>,
			T::AccountId: From<AccountId32>,
			 T: pallet_balances::Config,

	}

	collection_transfer{
	let (collection_id,dest_collection_id) = create_collection::<T,I>();
	set_collection_metadata::<T,I>();
	let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
	let sibling_account: AccountIdLookupOf<T> = T::Lookup::unlookup(convert_account_id::<T>(sibling_a_account()).into());
	let dest_location = MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())));
	// let dest_collection_id=  <T as pallet::Config<I>>::Helper::collection(0);
	let call = collection_config_with_all_settings_enabled::<T,I>();
	}: collection_transfer(SystemOrigin::Signed(caller.clone()),sibling_account.clone(),collection_id,dest_collection_id,dest_location,call)

	verify {
		assert_last_event::<T,I>(Event::CollectionTransferredSuccessfully::<T,I>.into());
	}

	nft_transfer{
	let (collection_id,dest_collection_id) = create_collection::<T,I>();
	let item_id =  mint_nft::<T,I>();
	 transfer_nft::<T,I>();
	 set_nft_metadata::<T,I>();
	let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
	 let dest_item_id = <T as pallet::Config<I>>::Helper::item(0);
	let mint_to_sibling: AccountIdLookupOf<T> = T::Lookup::unlookup(convert_account_id::<T>(sibling_a_account()));
	let new_account : T::AccountId = convert_account_id::<T>(sibling_b_account());
	let owner = T::Lookup::unlookup(new_account.clone());
	let dest_location = MultiLocation::new(1, X1(Parachain(2001)));
	}:nft_transfer(SystemOrigin::Signed(caller.clone()),collection_id,item_id,dest_collection_id,dest_item_id,mint_to_sibling,owner.clone(),dest_location)

	verify{
		assert_last_event::<T,I>(Event::NftOwnershipTransferred(  collection_id.clone(), item_id.clone(),owner.clone()).into());
	}

	transfer_collection_ownership{
		let (collection_id,dest_collection_id) = create_collection::<T,I>();
		let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
		let owner = T::Lookup::unlookup(caller.clone());
		let dest_location = MultiLocation::new(1, X1(Parachain(2001)));
		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup = T::Lookup::unlookup(target.clone());
		T::Currency::make_free_balance_be(&target, T::Currency::minimum_balance());

	}:transfer_collection_ownership(SystemOrigin::Signed(caller.clone()),owner.clone(),dest_collection_id,dest_location)

	verify{
		assert_last_event::<T,I>(Event::CollectionOwnershipTransferred::<T,I>( dest_collection_id.clone(),owner.clone() ).into());
	}

	transfer_multi_nfts{
		let (collection_id,dest_collection_id) = create_collection::<T,I>();
		let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
		 let item_id = vec![mint_nft::<T,I>()];
		let dest_item_id = dest_mint_nfts::<T,I>();
		insert_metadata::<T,I>();
		let mint_to_sibling: AccountIdLookupOf<T> = T::Lookup::unlookup(convert_account_id::<T>(sibling_a_account()));
		let dest_location = MultiLocation::new(1, Here);

	}:transfer_multi_nfts(SystemOrigin::Signed(caller.clone()),collection_id,item_id.clone(),dest_collection_id,dest_item_id.clone(),mint_to_sibling.clone(),dest_location)

	verify{
		for item_id in item_id.clone().iter() {
			assert_last_event::<T, I>(Event::NftSent::<T, I>(
				collection_id.clone(),
				*item_id,
				mint_to_sibling.clone()
			).into());
		}

	}
	transfer_nft_metadata{
		let (collection_id,dest_collection_id) = create_collection::<T,I>();
		let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
		let dest_item_id = dest_mint_nfts::<T,I>();
		let dest_location = MultiLocation::new(1, X1(Parachain(2001)));
		let data = insert_metadata::<T,I>();

	}:transfer_nft_metadata(SystemOrigin::Signed(caller.clone().into()),dest_collection_id.clone(),dest_item_id.clone(),dest_location)
	verify{
		for item_id in dest_item_id.clone(){
		assert_last_event::<T,I>(Event::ItemMetadataTransferred::<T,I>( dest_collection_id.clone(),item_id.clone(),data.clone() ).into());
	
		}
	}

	transfer_nfts_ownership{
		let (collection_id,dest_collection_id) = create_collection::<T,I>();
		let caller : T::AccountId = convert_account_id::<T>(sibling_a_account());
		let dest_collection_id = <T as pallet::Config<I>>::Helper::collection(0);
		let dest_item_id = dest_mint_nfts::<T,I>();
		let owner = T::Lookup::unlookup(caller.clone());
		let dest_location = MultiLocation::new(1, X1(Parachain(2001)));

	}:transfer_nfts_ownership(SystemOrigin::Signed(caller.clone()),owner.clone(),dest_collection_id.clone(),dest_item_id.clone(),dest_location)

	verify{
		for item_id in dest_item_id.clone(){
		assert_last_event::<T,I>(Event::NftOwnershipTransferred::<T,I>( dest_collection_id.clone(),item_id.clone(),owner.clone() ).into());
		}
	}

	 impl_benchmark_test_suite!(Xnft, crate::mock::para::new_test_ext(), crate::test::para::Test);
		//  impl_benchmark_test_suite!(Xnft, crate::mock::relay::new_test_ext(crate::mock::relay::GenesisConfigBuilder::default().build()), crate::test::para::Test);
	// impl_benchmark_test_suite!(Xnft, crate::mock::relay::new_test_ext(crate::mock::relay::GenesisConfigBuilder::default().build()), crate::test::relay::Test);
}
