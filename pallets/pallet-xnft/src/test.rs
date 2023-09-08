#![cfg(test)]
use super::*;
use crate::{
	mock::relay::Test,
	test::para::RuntimeOrigin,
};

use frame_support::{assert_noop, assert_ok,  traits::Currency};
pub use mock::*;
use pallet_nfts::types::*;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use xcm_simulator::TestExt;

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
fn collection_config_with_all_settings_enabled() -> CollectionConfigFor<Test> {
	CollectionConfig {
		settings: CollectionSettings::all_enabled(),
		max_supply: None,
		mint_settings: MintSettings::default(),
	}
}
fn set_balance() {
	Para1::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&sibling_a_account(), 1_000_000_000_000_000);
		let _ = RelayBalances::deposit_creating(&sibling_b_account(), 1_000_000_000_000);
		let _ = RelayBalances::deposit_creating(&ALICE, 1_000_000_000_000);
		let _ = RelayBalances::deposit_creating(&BOB, 1_000_000_000_000);
	});
}

struct TestSetup {
	collection_id: u32,
	dest_collection_id: u32,
	item_id: u32,
	dest_item_id: u32,
	sibling_account1: AccountId32,
	sibling_account2: AccountId32
}

fn initialize_param() -> TestSetup {
	TestSetup {
		collection_id: 0,
		dest_collection_id: 0,
		item_id: 0,
		dest_item_id: 0,
		sibling_account1: sibling_a_account(),
		sibling_account2: sibling_a_account(),
	}
}
struct MultiTestSetup {
	collection_id: u32,
	dest_collection_id: u32,
	item_id: Vec<u32>,
	dest_item_id: Vec<u32>,
	sibling_account1: AccountId32,
}

fn multi_initialize_param() -> MultiTestSetup {
	MultiTestSetup {
		collection_id: 0,
		dest_collection_id: 0,
		item_id: vec![0, 1, 2],
		dest_item_id: vec![0, 1, 2],
		sibling_account1: sibling_a_account(),
	}
}

fn create() {
	initialize_param();
	Para1::execute_with(|| {
		assert_ok!(NFT::create(
			RuntimeOrigin::signed(ALICE),
			ALICE,
			collection_config_with_all_settings_enabled(),
		));
	})
}

fn set_collection_metadata() {
	let fetch = initialize_param();
	Para1::execute_with(|| {
		assert_ok!(NFT::set_collection_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			bvec![0u8; 20],
		));
	})
}

fn collection_transfer() {
	let fetch = initialize_param();
	Para1::execute_with(|| {
		assert_ok!(ParaChain1::collection_transfer(
			RuntimeOrigin::signed(ALICE),
			fetch.sibling_account2.clone(),
			fetch.collection_id,
			fetch.dest_collection_id,
			MultiLocation::new(1, X1(Parachain(2001))),
			collection_config_with_all_settings_enabled(),
		));
	})
}

fn mint_nft() {
	let fetch = initialize_param();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			ALICE,
			None,
		));
	})
}

fn set_nft_metadata() {
	let fetch = initialize_param();
	Para1::execute_with(|| {
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			bvec![0u8; 20],
		));
	})
}

#[test]
fn collection_transfer_works() {
	TestNet::reset();
	let fetch = initialize_param();
	set_balance();
	create();
	set_collection_metadata();
	Para1::execute_with(|| {
		assert_ok!(ParaChain1::collection_transfer(
			RuntimeOrigin::signed(ALICE),
			fetch.sibling_account2,
			fetch.collection_id,
			fetch.dest_collection_id,
			MultiLocation::new(1, X1(Parachain(2001),)),
			collection_config_with_all_settings_enabled(),
		));
	});
}

#[test]
fn collection_transfer_owner_mismatch() {
	TestNet::reset();
	let fetch = initialize_param();
	set_balance();
	create();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::collection_transfer(
				RuntimeOrigin::signed(BOB),
				fetch.sibling_account2,
				fetch.collection_id,
				fetch.dest_collection_id,
				MultiLocation::new(1, X1(Parachain(2001))),
				collection_config_with_all_settings_enabled(),
			),
			Error::<Test>::NoTCollectionOwner
		);
	});
}

#[test]
fn collection_transfer_fails_for_nonexistent_collection() {
	TestNet::reset();
	set_balance();
	create();
	let fetch = initialize_param();
	let new_collection_id = 1;
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::collection_transfer(
				RuntimeOrigin::signed(ALICE),
				fetch.sibling_account2,
				new_collection_id,
				fetch.dest_collection_id,
				MultiLocation::new(1, X1(Parachain(2001),)),
				collection_config_with_all_settings_enabled(),
			),
			Error::<Test>::NoSuchCollectionId
		);
	});
}

#[test]
fn nft_transfer_works() {
	TestNet::reset();
	let fetch = initialize_param();
	let owner = BOB;
	set_balance();
	create();
	mint_nft();
	set_nft_metadata();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(ParaChain1::nft_transfer(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			fetch.dest_collection_id,
			fetch.dest_item_id,
			fetch.sibling_account1.clone(),
			owner,
			MultiLocation::new(1, X1(Parachain(2001))),
		));
	});
}

#[test]
fn nft_transfer_fails_owner_mismatch() {
	TestNet::reset();
	let owner = BOB;
	let fetch = initialize_param();
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::nft_transfer(
				RuntimeOrigin::signed(BOB),
				fetch.collection_id,
				fetch.item_id,
				fetch.dest_collection_id,
				fetch.dest_item_id,
				fetch.sibling_account2,
				owner,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoTNftOwner
		);
	});
}

#[test]
fn nft_transfer_fails_for_nonexistent_collection_id() {
	TestNet::reset();
	let fetch = initialize_param();
	let collection_id1 = 1;
	let owner1 = BOB;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::nft_transfer(
				RuntimeOrigin::signed(ALICE),
				collection_id1,
				fetch.item_id,
				fetch.dest_collection_id,
				fetch.dest_item_id,
				fetch.sibling_account2,
				owner1,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoSuchCollectionId
		);
	});
}

#[test]
fn nft_transfer_fails_for_nonexistent_item_id() {
	TestNet::reset();
	let fetch = initialize_param();
	let item_id1 = 1;
	let owner1 = BOB;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::nft_transfer(
				RuntimeOrigin::signed(ALICE),
				fetch.collection_id,
				item_id1,
				fetch.dest_collection_id,
				fetch.dest_item_id,
				fetch.sibling_account2,
				owner1,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoSuchItemId
		);
	});
}

#[test]
fn collection_ownership_transfer_works() {
	TestNet::reset();
	let fetch = initialize_param();
	set_balance();
	create();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(ParaChain1::transfer_collection_ownership(
			RuntimeOrigin::signed(ALICE),
			fetch.sibling_account2.clone(),
			fetch.collection_id,
			MultiLocation::new(1, X1(Parachain(2001),)),
		));
	});
}

#[test]
fn collection_ownership_transfer_fails_ownermismatch() {
	TestNet::reset();
	let fetch = initialize_param();
	set_balance();
	create();
	collection_transfer();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::transfer_collection_ownership(
				RuntimeOrigin::signed(BOB),
				fetch.sibling_account2.clone(),
				fetch.collection_id,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoTCollectionOwner
		);
	});
}

#[test]
fn collection_ownership_transfer_fails_no_collection_id() {
	TestNet::reset();
	let fetch = initialize_param();
	let collection_id1 = 1;
	set_balance();
	create();
	collection_transfer();
	Para1::execute_with(|| {
		assert_noop!(
			ParaChain1::transfer_collection_ownership(
				RuntimeOrigin::signed(ALICE),
				fetch.sibling_account2.clone(),
				collection_id1,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoSuchCollectionId
		);
	});
}

#[test]
fn multinft_transfer_works() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(ParaChain1::transfer_multi_nfts(
			RuntimeOrigin::signed(ALICE),
			multi_fetch.collection_id,
			multi_fetch.item_id,
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id,
			multi_fetch.sibling_account1.clone(),
			MultiLocation::new(1, X1(Parachain(2001))),
		));
	});
}

#[test]
fn multinft_transfer_fails_owner_mismatch() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_noop!(
			ParaChain1::transfer_multi_nfts(
				RuntimeOrigin::signed(BOB),
				multi_fetch.collection_id,
				multi_fetch.item_id,
				multi_fetch.dest_collection_id,
				multi_fetch.dest_item_id,
				multi_fetch.sibling_account1.clone(),
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoTNftOwner
		);
	});
}

#[test]
fn multinft_transfer_fails_for_nonexistent_collection_id() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	
	let item_id_1 = 1;
	let item_id_2 = 2;
	let new_collection_id = 3;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_noop!(
			ParaChain1::transfer_multi_nfts(
				RuntimeOrigin::signed(ALICE),
				new_collection_id,
				multi_fetch.item_id,
				multi_fetch.dest_collection_id,
				multi_fetch.dest_item_id,
				multi_fetch.sibling_account1.clone(),
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoSuchCollectionId
		);
	});
}

#[test]
fn multinft_transfer_fails_for_nonexistent_item_id() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	let new_item_id = vec![0, 1, 7];
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_noop!(
			ParaChain1::transfer_multi_nfts(
				RuntimeOrigin::signed(ALICE),
				multi_fetch.collection_id,
				new_item_id,
				multi_fetch.dest_collection_id,
				multi_fetch.dest_item_id,
				multi_fetch.sibling_account1.clone(),
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NoSuchItemId
		);
	});
}

#[test]
fn transfer_nfts_ownership_works() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(ParaChain1::transfer_multi_nfts(
			RuntimeOrigin::signed(ALICE),
			multi_fetch.collection_id,
			multi_fetch.item_id,
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id.clone(),
			multi_fetch.sibling_account1.clone(),
			MultiLocation::new(1, X1(Parachain(2001))),
		));
		assert_ok!(ParaChain1::transfer_nfts_ownership(
			RuntimeOrigin::signed(ALICE),
			BOB,
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id,
			MultiLocation::new(1, X1(Parachain(2001))),
		));
	});
}

#[test]
fn transfer_nfts_ownership_fails_limit_exceeds() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	let item_id_3 = 3;
	let new_item_id = vec![0, 1, 2, 3];
	let new_dest_item_id = vec![0, 1, 2, 3];
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_3,
			ALICE,
			None,
		));
		assert_noop!(
			ParaChain1::transfer_multi_nfts(
				RuntimeOrigin::signed(ALICE),
				multi_fetch.collection_id,
				new_item_id,
				multi_fetch.dest_collection_id,
				new_dest_item_id.clone(),
				multi_fetch.sibling_account1.clone(),
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::MaxItemCountExceeded
		);
		assert_noop!(
			ParaChain1::transfer_nfts_ownership(
				RuntimeOrigin::signed(ALICE),
				BOB,
				multi_fetch.dest_collection_id,
				new_dest_item_id,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::MaxItemCountExceeded
		);
	});
}

#[test]
fn transfer_nft_metadata_works() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			bvec![0u8; 20],
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			bvec![0u8; 20],
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			bvec![0u8; 20],
		));
		assert_ok!(ParaChain1::transfer_multi_nfts(
			RuntimeOrigin::signed(ALICE),
			multi_fetch.collection_id,
			multi_fetch.item_id,
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id.clone(),
			multi_fetch.sibling_account1.clone(),
			MultiLocation::new(1, X1(Parachain(2001))),
		));
		assert_ok!(ParaChain1::transfer_nft_metadata(
			RuntimeOrigin::signed(ALICE),
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id,
			MultiLocation::new(1, X1(Parachain(2001))),
		));
	});
}

#[test]
fn transfer_nft_metadata_fails_limit_exceeds() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	let item_id_3 = 3;
	let new_item_id = vec![0, 1, 2, 3];
	let new_dest_item_id = vec![0, 1, 2, 3];
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_3,
			ALICE,
			None,
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			bvec![0u8; 20],
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			bvec![0u8; 20],
		));
		assert_noop!(
			ParaChain1::transfer_multi_nfts(
				RuntimeOrigin::signed(ALICE),
				multi_fetch.collection_id,
				new_item_id,
				multi_fetch.dest_collection_id,
				new_dest_item_id.clone(),
				multi_fetch.sibling_account1.clone(),
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::MaxItemCountExceeded
		);
		assert_noop!(
			ParaChain1::transfer_nft_metadata(
				RuntimeOrigin::signed(ALICE),
				multi_fetch.dest_collection_id,
				new_dest_item_id,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::MaxDestItemCountExceeded
		);
	});
}

#[test]
fn transfer_nft_metadata_fails_ownermismatch() {
	TestNet::reset();
	let fetch = initialize_param();
	let multi_fetch = multi_initialize_param();
	let item_id_1 = 1;
	let item_id_2 = 2;
	set_balance();
	create();
	mint_nft();
	collection_transfer();
	Para1::execute_with(|| {
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			ALICE,
			None,
		));
		assert_ok!(NFT::mint(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			ALICE,
			None,
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			fetch.item_id,
			bvec![0u8; 20],
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_1,
			bvec![0u8; 20],
		));
		assert_ok!(NFT::set_metadata(
			RuntimeOrigin::signed(ALICE),
			fetch.collection_id,
			item_id_2,
			bvec![0u8; 20],
		));
		assert_ok!(ParaChain1::transfer_multi_nfts(
			RuntimeOrigin::signed(ALICE),
			multi_fetch.collection_id,
			multi_fetch.item_id,
			multi_fetch.dest_collection_id,
			multi_fetch.dest_item_id.clone(),
			multi_fetch.sibling_account1.clone(),
			MultiLocation::new(1, X1(Parachain(2001))),
		));
		assert_noop!(
			ParaChain1::transfer_nft_metadata(
				RuntimeOrigin::signed(BOB),
				multi_fetch.dest_collection_id,
				multi_fetch.dest_item_id,
				MultiLocation::new(1, X1(Parachain(2001))),
			),
			Error::<Test>::NotTheCollectionOwner
		);
	});
}
