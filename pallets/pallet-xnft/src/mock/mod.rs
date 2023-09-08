#![cfg(test)]

use super::*;
use sp_io::TestExternalities;
use sp_runtime::{ AccountId32  };
pub mod para;
pub mod relay;
use crate as xnft;
use relay::Hrmp;
use cumulus_primitives_core::ParaId;
use frame_support::traits::GenesisBuild;
use xcm_simulator::{ decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt };

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
//pub const ALICE: u64 = 1_000_000_000;
pub const INITIAL_BALANCE: u128 = 1_000_000_000;
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([1u8; 32]);
pub const DAVE: AccountId32 = AccountId32::new([1u8; 32]);

pub type Balance = u128;
pub type Amount = i128;

decl_test_parachain! {
	pub struct Para1 {
		Runtime = para::Test,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(4),
	}
}

decl_test_parachain! {
	pub struct Para2 {
		Runtime = para::Test,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(4),
	}
}

decl_test_relay_chain! {	
	pub struct Relay {
		Runtime = relay::Test,
		RuntimeCall = relay::RuntimeCall,
		RuntimeEvent = relay::RuntimeEvent,
		XcmConfig = relay::XcmConfig,
		MessageQueue = relay::MessageQueue,
		System = relay::System,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = Relay,
		parachains = vec![
			(2000, Para1),
			(2001, Para2),
		],
	}
}

pub fn sudo_establish_hrmp_channel(
	sender: ParaId,
	recipient: ParaId,
	max_capacity: u32,
	max_message_size: u32,
) -> sp_runtime::DispatchResult {

	Hrmp::init_open_channel(
		sender,
		recipient,
		max_capacity,
		max_message_size,
	);
	Hrmp::accept_open_channel(recipient, sender);
	Ok(())
}

fn sudo_establish_hrmp_channel_works()
 {
	relay_ext().execute_with(|| {
		let sender: ParaId = (2000).into();
		let para_a_origin: polkadot_runtime_parachains::Origin = 2000.into();
		let para_b_origin: polkadot_runtime_parachains::Origin = 2001.into();
		let recipient: ParaId = (2001).into();
		let max_capacity: u32 = 8;
		let max_message_size: u32 = 1048576;

		// Ensure the root origin is used
		let origin =ALICE;
		// Call the function with the parameters
		let result = sudo_establish_hrmp_channel(
			
			sender,
			recipient,
			max_capacity,
			max_message_size
		);

		Hrmp::hrmp_init_open_channel(para_a_origin.into(), recipient, 8, 1048576).unwrap_or_default();
		Hrmp::hrmp_accept_open_channel(para_b_origin.into(), sender).unwrap_or_default();
		// Hrmp::Config::assert_storage_consistency_exhaustive;
		// <Test as polkadot_runtime_parachains::Hrmp::Config>::assert_storage_consistency_exhaustive;

		// Assert that the operation was successful
		frame_support::assert_ok!(result);

		// Check if the HRMP channel is initialized as expected
		// let channel_exists = hrmp::Pallet::<Test>::is_channel_initialized(sender, recipient);
		// assert_eq!(channel_exists, true);
	});
}
pub type RelayBalances = pallet_balances::Pallet<relay::Test>;
pub type ParaChain1 = xnft::Pallet<para::Test>;
pub type NFT = pallet_nfts::Pallet<para::Test>;
pub fn para_ext(para_id: u32) -> TestExternalities {
    use para::{ System, Test };

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
    <parachain_info::GenesisConfig as GenesisBuild<Test, _>>
        ::assimilate_storage(&parachain_info_config, &mut t)
        .unwrap();

    let mut ext = TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn para_teleport_ext(para_id: u32) -> TestExternalities {
    use para::{ System, Test };

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
    <parachain_info::GenesisConfig as GenesisBuild<Test, _>>
        ::assimilate_storage(&parachain_info_config, &mut t)
        .unwrap();

    let mut ext = TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}


pub fn relay_ext() -> sp_io::TestExternalities {
    use relay::{ System, Test };

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    (pallet_balances::GenesisConfig::<Test> { balances: vec![(ALICE, INITIAL_BALANCE)] })
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
