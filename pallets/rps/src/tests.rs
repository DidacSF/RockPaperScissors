use crate::{mock::*, *, Error};

use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;
use crate::Error::InsufficientBetAmount;

const ALICE: <Test as frame_system::Config>::AccountId = 1_u64;
const BOB: <Test as frame_system::Config>::AccountId = 2_u64;
const CHARLIE: <Test as frame_system::Config>::AccountId = 3_u64;

const BET_AMOUNT: u64 = 1_000;

#[test]
fn should_create_challenge_correctly() {
	new_test_ext().execute_with(|| {
		let challenge_creator = ALICE;

		assert_ok!(RpsModule::create_challenge(Origin::signed(challenge_creator), BET_AMOUNT), ());
		// TODO: Investigate how to check events
		/*assert_eq!(
			last_event(),
			mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
		);*/
		assert_eq!(RpsModule::next_challenge_id(), 1_u64);

		let challenge = ChallengeState::Open(OpenChallenge {
			challenger: ALICE,
			bet_amount: 1000,
		});
		assert_eq!(RpsModule::challenge_store(0_u64), Some(challenge));
	});
}

#[test]
fn should_fail_to_create_challenge_with_unsigned_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(RpsModule::create_challenge(Origin::none(), BET_AMOUNT), BadOrigin);
	});
}

#[test]
fn should_fail_to_create_challenge_with_insufficient_bet_amount() {
	new_test_ext().execute_with(|| {
		let challenge_creator = ALICE;

		assert_noop!(RpsModule::create_challenge(Origin::signed(challenge_creator), 0), Error::<Test>::InsufficientBetAmount);
	});
}


/*fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(RpsModule::cause_error(Origin::signed(1)), Error::<Test>::NoneValue);
	});
}*/
