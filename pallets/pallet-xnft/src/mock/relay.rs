#![cfg(test)]
use crate::{
	self as pallet_xnft,
	test::{relay::currency::{CENTS, MILLICENTS}, ALICE},
};
use xcm_builder::SovereignSignedViaLocation;
use cumulus_primitives_core::relay_chain::CandidateHash;
use xcm_builder::MintLocation;
use sp_runtime::Perbill;
use frame_support::weights::IdentityFee;
use polkadot_runtime_parachains::origin as parachains_origin;
use frame_support::weights::constants::ExtrinsicBaseWeight;
use xcm_builder::ChildParachainAsNative;
use smallvec::smallvec;
use frame_support::weights::WeightToFeePolynomial;
use frame_support::weights::WeightToFeeCoefficient;
use frame_support::weights::WeightToFeeCoefficients;
use xcm_builder::UsingComponents;
use polkadot_runtime_common::ToAuthor;
use xcm_builder::AccountId32Aliases;
use xcm_builder::SignedAccountId32AsNative;
use frame_support::{ traits::AsEnsureOriginWithArg};
use polkadot_runtime_common::{
	paras_sudo_wrapper,
	xcm_sender::{ChildParachainRouter, ExponentialPrice},
};
use xcm_builder::ChildSystemParachainAsSuperuser;
pub use polkadot_runtime_parachains::hrmp;
use polkadot_runtime_parachains::{
	dmp as parachains_dmp, hrmp::HrmpChannels, 
};

use frame_support::{
	construct_runtime,
	parameter_types,
	traits::{
		 ConstU32, ConstU64, ConstU128, Everything, GenesisBuild, Nothing,
		ProcessMessage, ProcessMessageError,
	},
	weights::{ Weight, WeightMeter},
	
};

use crate::test::relay::currency::DOLLARS;
use cumulus_primitives_core::{
	relay_chain::{AuthorityDiscoveryId, SessionIndex, ValidatorIndex},
	ChannelStatus, GetChannelInfo, ParaId,
};
use frame_support::traits::ValidatorSetWithIdentification;
use frame_system::{EnsureRoot};
use sp_runtime::{transaction_validity::TransactionPriority, Permill};
use std::{cell::RefCell, collections::HashMap};
pub mod currency {
	use node_primitives::Balance;

	pub const MILLICENTS: Balance = 1_000_000_000;
	pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	pub const DOLLARS: Balance = 100 * CENTS;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}
use sp_runtime::{
	generic,
	traits::{ BlakeTwo256, IdentityLookup},
	 MultiSignature,
};
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as sp_runtime::traits::Verify>::Signer;
pub type AccountId = <AccountPublic as sp_runtime::traits::IdentifyAccount>::AccountId;
use crate::test::para::ParachainInfo;
use frame_support::traits::ValidatorSet;
use sp_core::H256;
use xcm_builder::{EnsureXcmOrigin, NativeAsset};
//pub type AccountId = u64;
use pallet_nfts::PalletFeatures;
use polkadot_runtime_parachains::{disputes, inclusion, paras, scheduler, session_info};

use polkadot_runtime_parachains::{
	configuration,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	origin, shared,
};
pub type BlockNumber = u32;
pub type Index = u32;
use xcm::v3::prelude::*;
use xcm_builder::{
	 AllowTopLevelPaidExecutionFrom, 
	ChildParachainConvertsVia, FixedWeightBounds,
	  SignedToAccountId32,
	TakeWeightCredit,
};
use xcm_executor::XcmExecutor;

type Origin = <Test as frame_system::Config>::RuntimeOrigin;


type Balance = u128;

pub fn root_user() -> Origin {
	RuntimeOrigin::root()
}
pub fn who(who: AccountId) -> Origin {
	RuntimeOrigin::signed(who)
}

pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Test>,
	frame_system::CheckSpecVersion<Test>,
	frame_system::CheckTxVersion<Test>,
	frame_system::CheckGenesis<Test>,
	frame_system::CheckEra<Test>,
	frame_system::CheckNonce<Test>,
	frame_system::CheckWeight<Test>,
);

type UncheckedExtrinsics = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsics,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances,
		ParasOrigin: origin,
		MessageQueue: pallet_message_queue,
		XcmPallet: pallet_xcm,
		ParaInclusion: inclusion,
		Paras: paras,
		Xnft: pallet_xnft,
		Disputes: disputes,
		Scheduler: scheduler,
		Configuration: configuration,
		ParasShared: shared,
		ParasSudoWrapperCall:paras_sudo_wrapper,
		Dmp: parachains_dmp,
		NFT:pallet_nfts,
		CumulusXcm: cumulus_pallet_xcm,
		DmpQueue: cumulus_pallet_dmp_queue,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		Hrmp: hrmp,
	}

);
impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsics;
	type OverarchingCall = RuntimeCall;
}

#[derive(Debug)]
pub struct GenesisConfigBuilder {
	hrmp_channel_max_capacity: u32,
	hrmp_channel_max_message_size: u32,
	hrmp_max_paras_outbound_channels: u32,
	hrmp_max_paras_inbound_channels: u32,
	hrmp_max_message_num_per_candidate: u32,
	hrmp_channel_max_total_size: u32,
	hrmp_sender_deposit: Balance,
	hrmp_recipient_deposit: Balance,
}

impl Default for GenesisConfigBuilder {
	fn default() -> Self {
		Self {
			hrmp_channel_max_capacity: 2,
			hrmp_channel_max_message_size: 8,
			hrmp_max_paras_outbound_channels: 2,
			hrmp_max_paras_inbound_channels: 2,
			hrmp_max_message_num_per_candidate: 2,
			hrmp_channel_max_total_size: 16,
			hrmp_sender_deposit: 100,
			hrmp_recipient_deposit: 100,
		}
	}
}

impl GenesisConfigBuilder {
	pub fn build(self) -> MockGenesisConfig {
		let mut genesis = default_genesis_config();
		let channel = sudo_establish_hrmp_channel_works();
		let config = &mut genesis.configuration.config;
		config.hrmp_channel_max_capacity = self.hrmp_channel_max_capacity;
		config.hrmp_channel_max_message_size = self.hrmp_channel_max_message_size;
		config.hrmp_max_parachain_outbound_channels = self.hrmp_max_paras_outbound_channels;
		config.hrmp_max_parachain_inbound_channels = self.hrmp_max_paras_inbound_channels;
		config.hrmp_max_message_num_per_candidate = self.hrmp_max_message_num_per_candidate;
		config.hrmp_channel_max_total_size = self.hrmp_channel_max_total_size;
		config.hrmp_sender_deposit = self.hrmp_sender_deposit;
		config.hrmp_recipient_deposit = self.hrmp_recipient_deposit;
		genesis
	}
}

fn default_genesis_config() -> MockGenesisConfig {
	MockGenesisConfig {
		configuration: polkadot_runtime_parachains::configuration::GenesisConfig {
			config: polkadot_runtime_parachains::configuration::HostConfiguration {
				max_downward_message_size: 1024,
				..Default::default()
			},
		},
		
		..Default::default()
	}
}

fn channel_exists(sender: ParaId, recipient: ParaId) -> bool {
	HrmpChannels::<Test>::get(&primitives::HrmpChannelId { sender, recipient }).is_some()
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
	new_test_ext(MockGenesisConfig::default()).execute_with(|| {
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

impl paras_sudo_wrapper::Config for Test {}
pub struct ChannelInfo;
impl GetChannelInfo for ChannelInfo {
	fn get_channel_status(_id: ParaId) -> ChannelStatus {
		ChannelStatus::Ready(10, 10)
	}
	fn get_channel_max(_id: ParaId) -> Option<usize> {
		Some(usize::max_value())
	}
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	//type Nonce = u64;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	//type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const CollectionDeposit: Balance = 100 * DOLLARS;
	pub const ItemDeposit: Balance = 1 * DOLLARS;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
	pub const ApprovalsLimit: u32 = 20;
	pub const ItemAttributesApprovalsLimit: u32 = 20;
	pub const MaxTips: u32 = 10;

}

parameter_types! {
	pub Features: PalletFeatures = PalletFeatures::all_enabled();
	pub const MaxAttributesPerCall: u32 = 10;
}

impl pallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Locker = ();
	type CollectionDeposit = ConstU128<2>;
	type ItemDeposit = ConstU128<1>;
	type MetadataDepositBase = ConstU128<1>;
	type AttributeDepositBase = ConstU128<1>;
	type DepositPerByte = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type KeyLimit = ConstU32<50>;
	type ValueLimit = ConstU32<50>;
	type ApprovalsLimit = ConstU32<10>;
	type ItemAttributesApprovalsLimit = ConstU32<2>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = ();
	type MaxAttributesPerCall = ConstU32<2>;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as sp_runtime::traits::Verify>::Signer;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type HoldIdentifier = ();
	type MaxHolds = ();
}
parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

pub struct TestNextSessionRotation;

impl frame_support::traits::EstimateNextSessionRotation<u32> for TestNextSessionRotation {
	fn average_session_length() -> u32 {
		10
	}

	fn estimate_current_session_progress(_now: u32) -> (Option<Permill>, Weight) {
		(None, Weight::zero())
	}

	fn estimate_next_session_rotation(_now: u32) -> (Option<u32>, Weight) {
		(None, Weight::zero())
	}
}

impl paras::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = polkadot_runtime_parachains::paras::TestWeightInfo;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = TestNextSessionRotation;
}

thread_local! {
	pub static BACKING_REWARDS: RefCell<HashMap<ValidatorIndex, usize>>
		= RefCell::new(HashMap::new());

	pub static AVAILABILITY_REWARDS: RefCell<HashMap<ValidatorIndex, usize>>
		= RefCell::new(HashMap::new());
}

pub struct TestRewardValidators;

impl inclusion::RewardValidators for TestRewardValidators {
	fn reward_backing(v: impl IntoIterator<Item = ValidatorIndex>) {
		BACKING_REWARDS.with(|r| {
			let mut r = r.borrow_mut();
			for i in v {
				*r.entry(i).or_insert(0) += 1;
			}
		})
	}
	fn reward_bitfields(v: impl IntoIterator<Item = ValidatorIndex>) {
		AVAILABILITY_REWARDS.with(|r| {
			let mut r = r.borrow_mut();
			for i in v {
				*r.entry(i).or_insert(0) += 1;
			}
		})
	}
}

impl inclusion::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = Disputes;
	type RewardValidators = TestRewardValidators;
	type MessageQueue = MessageQueue;
}

pub struct ValidatorIdOf;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
	fn convert(a: AccountId) -> Option<AccountId> {
		Some(a)
	}
}

pub struct MockValidatorSet;

impl ValidatorSet<AccountId> for MockValidatorSet {
	type ValidatorId = AccountId;
	type ValidatorIdOf = ValidatorIdOf;
	fn session_index() -> SessionIndex {
		0
	}
	fn validators() -> Vec<Self::ValidatorId> {
		Vec::new()
	}
}

pub struct FoolIdentificationOf;
impl sp_runtime::traits::Convert<AccountId, Option<()>> for FoolIdentificationOf {
	fn convert(_: AccountId) -> Option<()> {
		Some(())
	}
}

impl ValidatorSetWithIdentification<AccountId> for MockValidatorSet {
	type Identification = ();
	type IdentificationOf = FoolIdentificationOf;
}

impl scheduler::Config for Test {}

impl disputes::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators = Self;
	type SlashingHandler = Self;
	type WeightInfo = disputes::TestWeightInfo;
}

thread_local! {
	pub static REWARD_VALIDATORS: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_VALIDATORS_FOR: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_VALIDATORS_AGAINST: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_BACKERS_FOR: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
}

impl disputes::RewardValidators for Test {
	fn reward_dispute_statement(
		session: SessionIndex,
		validators: impl IntoIterator<Item = ValidatorIndex>,
	) {
		REWARD_VALIDATORS.with(|r| r.borrow_mut().push((session, validators.into_iter().collect())))
	}
}

impl disputes::SlashingHandler<BlockNumber> for Test {
	fn punish_for_invalid(
		session: SessionIndex,
		_: CandidateHash,
		losers: impl IntoIterator<Item = ValidatorIndex>,
		backers: impl IntoIterator<Item = ValidatorIndex>,
	) {
		PUNISH_VALIDATORS_FOR
			.with(|r| r.borrow_mut().push((session, losers.into_iter().collect())));
		PUNISH_BACKERS_FOR.with(|r| r.borrow_mut().push((session, backers.into_iter().collect())));
	}

	fn punish_against_valid(
		session: SessionIndex,
		_: CandidateHash,
		losers: impl IntoIterator<Item = ValidatorIndex>,
		_backers: impl IntoIterator<Item = ValidatorIndex>,
	) {
		PUNISH_VALIDATORS_AGAINST
			.with(|r| r.borrow_mut().push((session, losers.into_iter().collect())))
	}

	fn initializer_initialize(_now: BlockNumber) -> Weight {
		Weight::zero()
	}

	fn initializer_finalize() {}

	fn initializer_on_new_session(_: SessionIndex) {}
}

thread_local! {
	pub static DISCOVERY_AUTHORITIES: RefCell<Vec<AuthorityDiscoveryId>> = RefCell::new(Vec::new());
}

pub fn discovery_authorities() -> Vec<AuthorityDiscoveryId> {
	DISCOVERY_AUTHORITIES.with(|r| r.borrow().clone())
}

impl session_info::AuthorityDiscoveryConfig for Test {
	fn authorities() -> Vec<AuthorityDiscoveryId> {
		discovery_authorities()
	}
}

impl session_info::Config for Test {
	type ValidatorSet = MockValidatorSet;
}

impl hrmp::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = pallet_balances::Pallet<Test>;
	type WeightInfo = hrmp::TestWeightInfo;
}

impl shared::Config for Test {}

impl configuration::Config for Test {
	type WeightInfo = configuration::TestWeightInfo;
}

impl cumulus_pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ChannelInfo;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = ();
	type WeightInfo = ();
	type PriceForSiblingDelivery = ();
}
parameter_types! {
	pub const TokenLocation: MultiLocation = Here.into_location();
	pub const ThisNetwork: NetworkId = NetworkId::Rococo;
	pub UniversalLocation: InteriorMultiLocation = ThisNetwork::get().into();
	pub CheckAccount: AccountId = XcmPallet::check_account();
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);
}

pub type SovereignAccountOf = (ChildParachainConvertsVia<ParaId, AccountId>,);
parameter_types! {
	pub const MaxAssetsForTransfer: usize = 2;
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
}

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = Concrete(TokenLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

pub type XcmRouter = (
	// Only one router so far - use DMP to communicate with child parachains.
	ChildParachainRouter<
		Test,
		XcmPallet,
		ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>,
	>,
);

pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(10, 10);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}
impl cumulus_pallet_dmp_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

pub type LocationConverter =
	(ChildParachainConvertsVia<ParaId, AccountId>, AccountId32Aliases<ThisNetwork, AccountId>);


type LocalOriginConverter = (
	// A `Signed` origin of the sovereign account that the original location controls.
	SovereignSignedViaLocation<LocationConverter, RuntimeOrigin>,
	// A child parachain, natively expressed, has the `Parachain` origin.
	ChildParachainAsNative<parachains_origin::Origin, RuntimeOrigin>,
	// The AccountId32 location type can be expressed natively as a `Signed` origin.
	SignedAccountId32AsNative<ThisNetwork, RuntimeOrigin>,
	// A system child parachain, expressed as a Superuser, converts to the `Root` origin.
	ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
);

impl parachains_dmp::Config for Test {}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		let p = currency::CENTS;
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

parameter_types! {
	pub const Roc: MultiAssetFilter = Wild(AllOf { fun: WildFungible, id: Concrete(TokenLocation::get()) });
	pub const Para: MultiLocation = Parachain(2000).into_location();
	pub const Para2: MultiLocation = Parachain(2001).into_location();
	pub const OurchainPara: (MultiAssetFilter, MultiLocation) = (Roc::get(), Para::get());
	pub const OurchainPara2: (MultiAssetFilter, MultiLocation) = (Roc::get(), Para2::get());

}

pub type TrustedTeleporters = (xcm_builder::Case<OurchainPara>, xcm_builder::Case<OurchainPara2>);


pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = NativeAsset;
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = ();
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = ();
	type AssetTransactor = ();
	type Trader = UsingComponents<IdentityFee<Balance>, TokenLocation, AccountId, Balances, ()>;
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, ThisNetwork>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

impl origin::Config for Test {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 16;
}

pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<Junction, xcm_executor::XcmExecutor<XcmConfig>, RuntimeCall>::process_message(
			message,
			Junction::Parachain(para.into()),
			meter,
			id,
		)
	}
}

impl pallet_message_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type MessageProcessor = MessageProcessor;
	type QueueChangeHandler = ();
	//type QueuePausedQuery = ();
	type WeightInfo = ();
}

impl pallet_xnft::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmSender = XcmRouter;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

pub fn new_test_ext(state: MockGenesisConfig) -> sp_io::TestExternalities {
	use sp_keystore::{testing::MemoryKeystore, KeystoreExt, KeystorePtr};
	use sp_std::sync::Arc;

	sp_tracing::try_init_simple();

	BACKING_REWARDS.with(|r| r.borrow_mut().clear());
	AVAILABILITY_REWARDS.with(|r| r.borrow_mut().clear());

	let mut t = state.system.build_storage::<Test>().unwrap();
	state.configuration.assimilate_storage(&mut t).unwrap();
	GenesisBuild::<Test>::assimilate_storage(&state.paras, &mut t).unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

	ext
}
pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		storage.into()
	}
}

pub fn assert_last_events<E>(generic_events: E)
where
	E: DoubleEndedIterator<Item = RuntimeEvent> + ExactSizeIterator,
{
	for (i, (got, want)) in frame_system::Pallet::<Test>::events()
		.into_iter()
		.rev()
		.map(|e| e.event)
		.zip(generic_events.rev().map(<Test as frame_system::Config>::RuntimeEvent::from))
		.rev()
		.enumerate()
	{
		assert_eq!((i, got), (i, want));
	}
}

#[derive(Default)]
pub struct MockGenesisConfig {
	pub system: frame_system::GenesisConfig,
	pub configuration: configuration::GenesisConfig<Test>,
	pub paras: paras::GenesisConfig,
}

