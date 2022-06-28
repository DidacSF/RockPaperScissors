use crate::{mock::*, Error, *};

use frame_support::{assert_noop, assert_ok};
use frame_system::AccountInfo;
use pallet_balances::AccountData;
use sp_runtime::DispatchError::BadOrigin;

pub type MockAccountId = <Test as frame_system::Config>::AccountId;

const ALICE: MockAccountId = 1_u64;
const BOB: MockAccountId = 2_u64;
const CHARLIE: MockAccountId = 3_u64;

const ENDOWED_ACCOUNTS: [MockAccountId; 3] = [ALICE, BOB, CHARLIE];
const ENDOWMENT_AMOUNT: u64 = 1_000_000_u64;

const BET_AMOUNT: u64 = 1_000;

fn create_challenge(challenger: MockAccountId) -> u64 {
	assert_ok!(RpsModule::create_challenge(Origin::signed(challenger), BET_AMOUNT), ());

	let challenge = ChallengeState::Open(OpenChallenge { challenger, bet_amount: BET_AMOUNT });

	let challenge_id = RpsModule::next_challenge_id() - 1;

	assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));

	challenge_id
}

fn create_accepted_challenge(challenger: MockAccountId, rival: MockAccountId) -> u64 {
	let challenge_id = create_challenge(challenger);

	assert_ok!(RpsModule::enter_challenge(Origin::signed(rival), challenge_id), ());

	let challenge =
		ChallengeState::Accepted(AcceptedChallenge { challenger, bet_amount: 1000, rival });

	assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));

	challenge_id
}

fn create_played_challenge(
	challenger: MockAccountId,
	rival: MockAccountId,
	challenger_play: (ChallengePlay, u64),
	rival_play: (ChallengePlay, u64),
) -> u64 {
	let challenge_id = create_accepted_challenge(challenger, rival);

	let challenger_hash = challenger_play.0.generate_hash(challenger_play.1);

	assert_ok!(
		RpsModule::play_challenge(
			Origin::signed(challenger),
			challenge_id,
			challenger_play.0,
			challenger_play.1
		),
		()
	);

	// TODO: Investigate how to check events
	/*assert_eq!(
		last_event(),
		mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
	);*/

	assert_eq!(RpsModule::challenge_plays_store(challenge_id, challenger), Some(challenger_hash));

	let account_state_after_play = AccountInfo {
		nonce: 0,
		consumers: 0,
		providers: 1,
		sufficients: 0,
		data: AccountData {
			free: ENDOWMENT_AMOUNT - BET_AMOUNT,
			reserved: BET_AMOUNT,
			misc_frozen: 0,
			fee_frozen: 0,
		},
	};

	assert_eq!(System::account(challenger), account_state_after_play);

	let rival_hash = rival_play.0.generate_hash(rival_play.1);

	assert_ok!(
		RpsModule::play_challenge(Origin::signed(rival), challenge_id, rival_play.0, rival_play.1),
		()
	);

	// TODO: Investigate how to check events
	/*assert_eq!(
		last_event(),
		mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
	);*/

	// TODO: Investigate how to check events
	/*assert_eq!(
		last_event(),
		mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
	);*/

	assert_eq!(RpsModule::challenge_plays_store(challenge_id, rival), Some(rival_hash));
	assert_eq!(System::account(rival), account_state_after_play);

	challenge_id
}

#[cfg(test)]
mod create_challenge {
	use super::*;

	#[test]
	fn should_create_challenge() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_creator = ALICE;

			assert_ok!(
				RpsModule::create_challenge(Origin::signed(challenge_creator), BET_AMOUNT),
				()
			);
			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/
			assert_eq!(RpsModule::next_challenge_id(), 1_u64);

			let challenge =
				ChallengeState::Open(OpenChallenge { challenger: ALICE, bet_amount: 1000 });
			assert_eq!(RpsModule::challenge_store(0_u64), Some(challenge));
		});
	}

	#[test]
	fn should_fail_to_create_challenge_with_unsigned_origin() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			assert_noop!(RpsModule::create_challenge(Origin::none(), BET_AMOUNT), BadOrigin);
		});
	}

	#[test]
	fn should_fail_to_create_challenge_with_insufficient_bet_amount() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_creator = ALICE;

			assert_noop!(
				RpsModule::create_challenge(Origin::signed(challenge_creator), 0),
				Error::<Test>::InsufficientBetAmount
			);
		});
	}
}

#[cfg(test)]
mod enter_challenge {
	use super::*;

	#[test]
	fn should_enter_challenge() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_challenge(ALICE);

			assert_ok!(RpsModule::enter_challenge(Origin::signed(BOB), challenge_id), ());

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			let challenge = ChallengeState::Accepted(AcceptedChallenge {
				challenger: ALICE,
				bet_amount: BET_AMOUNT,
				rival: BOB,
			});

			assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));
		});
	}

	#[test]
	fn should_fail_to_enter_challenge_created_by_oneself() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_challenge(ALICE);

			assert_noop!(
				RpsModule::enter_challenge(Origin::signed(ALICE), challenge_id),
				Error::<Test>::CannotChallengeOneself
			);
		});
	}

	#[test]
	fn should_fail_to_enter_non_existent_challenge() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_challenge(ALICE);

			assert_noop!(
				RpsModule::enter_challenge(Origin::signed(BOB), challenge_id + 10),
				Error::<Test>::ChallengeNotFound
			);
		});
	}

	#[test]
	fn should_fail_to_enter_non_open_challenge() {
		new_test_ext(&[], ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_challenge(ALICE);

			assert_ok!(RpsModule::enter_challenge(Origin::signed(BOB), challenge_id), ());

			let challenge = ChallengeState::Accepted(AcceptedChallenge {
				challenger: ALICE,
				bet_amount: BET_AMOUNT,
				rival: BOB,
			});
			assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));

			assert_noop!(
				RpsModule::enter_challenge(Origin::signed(CHARLIE), challenge_id),
				Error::<Test>::ChallengeNotOpen
			);
		});
	}
}

#[cfg(test)]
mod play_challenge {
	use super::*;

	#[test]
	fn should_play_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let rival = BOB;

			let challenge_id = create_accepted_challenge(challenger, rival);

			let challenger_play = ChallengePlay::Paper;
			let challenger_secret = 319_u64;
			let challenger_hash = challenger_play.generate_hash(challenger_secret);

			assert_ok!(
				RpsModule::play_challenge(
					Origin::signed(challenger),
					challenge_id,
					challenger_play,
					challenger_secret
				),
				()
			);

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			assert_eq!(
				RpsModule::challenge_plays_store(challenge_id, challenger),
				Some(challenger_hash)
			);

			let account_state_after_play = AccountInfo {
				nonce: 0,
				consumers: 0,
				providers: 1,
				sufficients: 0,
				data: AccountData {
					free: ENDOWMENT_AMOUNT - BET_AMOUNT,
					reserved: BET_AMOUNT,
					misc_frozen: 0,
					fee_frozen: 0,
				},
			};

			assert_eq!(System::account(challenger), account_state_after_play);

			let rival_play = ChallengePlay::Scissors;
			let rival_secret = 37515_u64;
			let rival_hash = rival_play.generate_hash(rival_secret);

			assert_ok!(
				RpsModule::play_challenge(
					Origin::signed(rival),
					challenge_id,
					rival_play,
					rival_secret
				),
				()
			);

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			assert_eq!(RpsModule::challenge_plays_store(challenge_id, rival), Some(rival_hash));
			assert_eq!(System::account(rival), account_state_after_play);
		});
	}

	#[test]
	fn should_fail_to_play_in_nonexistent_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_accepted_challenge(ALICE, BOB);

			let challenger_play = ChallengePlay::Scissors;
			let challenger_secret = 9921_u64;

			assert_noop!(
				RpsModule::play_challenge(
					Origin::signed(BOB),
					challenge_id + 10,
					challenger_play,
					challenger_secret
				),
				Error::<Test>::ChallengeNotFound
			);
		});
	}

	#[test]
	fn should_fall_to_play_twice_in_the_same_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_accepted_challenge(ALICE, BOB);

			let challenger_play = ChallengePlay::Scissors;
			let challenger_secret = 9921_u64;

			assert_ok!(
				RpsModule::play_challenge(
					Origin::signed(BOB),
					challenge_id,
					challenger_play,
					challenger_secret
				),
				()
			);

			assert_noop!(
				RpsModule::play_challenge(
					Origin::signed(BOB),
					challenge_id,
					challenger_play,
					challenger_secret
				),
				Error::<Test>::ChallengeAlreadyPlayed
			);
		});
	}

	#[test]
	fn should_fail_to_play_in_non_accepted_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_challenge(ALICE);

			let challenger_play = ChallengePlay::Rock;
			let challenger_secret = 15235_u64;

			assert_noop!(
				RpsModule::play_challenge(
					Origin::signed(BOB),
					challenge_id,
					challenger_play,
					challenger_secret
				),
				Error::<Test>::ChallengeStateForbidsPlay
			);
		});
	}

	#[test]
	fn should_fail_to_play_in_non_participating_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenge_id = create_accepted_challenge(ALICE, BOB);

			let challenger_play = ChallengePlay::Paper;
			let challenger_secret = 98571_u64;

			assert_noop!(
				RpsModule::play_challenge(
					Origin::signed(CHARLIE),
					challenge_id,
					challenger_play,
					challenger_secret
				),
				Error::<Test>::CannotPlayInNonParticipatingChallenge
			);
		});
	}
}

#[cfg(test)]
mod resolve_challenge {
	use super::*;

	#[test]
	fn should_resolve_challenge_with_winner() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let rival = BOB;
			let rival_play = (ChallengePlay::Rock, 481_u64);

			let challenge_id =
				create_played_challenge(challenger, rival, challenger_play, rival_play);

			assert_ok!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					rival_play.0,
					rival_play.1,
					challenge_id
				),
				()
			);

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			let challenge = ChallengeState::Finished(FinishedChallenge {
				challenger,
				bet_amount: BET_AMOUNT,
				rival,
				winner: Some(rival),
			});

			assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));

			let winner_account_state_after_reveal = AccountInfo {
				nonce: 0,
				consumers: 0,
				providers: 1,
				sufficients: 0,
				data: AccountData {
					free: ENDOWMENT_AMOUNT + BET_AMOUNT,
					reserved: 0,
					misc_frozen: 0,
					fee_frozen: 0,
				},
			};

			assert_eq!(System::account(rival), winner_account_state_after_reveal);

			let loser_account_state_after_reveal = AccountInfo {
				nonce: 0,
				consumers: 0,
				providers: 1,
				sufficients: 0,
				data: AccountData {
					free: ENDOWMENT_AMOUNT - BET_AMOUNT,
					reserved: 0,
					misc_frozen: 0,
					fee_frozen: 0,
				},
			};

			assert_eq!(System::account(challenger), loser_account_state_after_reveal);
		});
	}

	#[test]
	fn should_resolve_challenge_with_no_winner() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Rock, 57832_u64);

			let rival = BOB;
			let rival_play = (ChallengePlay::Rock, 481_u64);

			let challenge_id =
				create_played_challenge(challenger, rival, challenger_play, rival_play);

			assert_ok!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					rival_play.0,
					rival_play.1,
					challenge_id
				),
				()
			);

			// TODO: Investigate how to check events
			/*assert_eq!(
				last_event(),
				mock::Event::RpsModule(crate::Event::ChallengeCreated(1_u64, challenge_creator, bet_amount)),
			);*/

			let challenge = ChallengeState::Finished(FinishedChallenge {
				challenger,
				bet_amount: BET_AMOUNT,
				rival,
				winner: None,
			});

			assert_eq!(RpsModule::challenge_store(challenge_id), Some(challenge));

			let account_state_after_draw = AccountInfo {
				nonce: 0,
				consumers: 0,
				providers: 1,
				sufficients: 0,
				data: AccountData {
					free: ENDOWMENT_AMOUNT,
					reserved: 0,
					misc_frozen: 0,
					fee_frozen: 0,
				},
			};

			assert_eq!(System::account(rival), account_state_after_draw);
			assert_eq!(System::account(challenger), account_state_after_draw);
		});
	}

	#[test]
	fn should_fail_to_resolve_nonexistent_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let rival = BOB;
			let rival_play = (ChallengePlay::Rock, 481_u64);

			let challenge_id =
				create_played_challenge(challenger, rival, challenger_play, rival_play);

			assert_noop!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					rival_play.0,
					rival_play.1,
					challenge_id + 10
				),
				Error::<Test>::ChallengeNotFound
			);
		});
	}

	#[test]
	fn should_fail_to_resolve_non_participating_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let rival = BOB;
			let rival_play = (ChallengePlay::Rock, 481_u64);

			let challenge_id =
				create_played_challenge(challenger, rival, challenger_play, rival_play);

			assert_noop!(
				RpsModule::reveal_challenge_results(
					Origin::signed(CHARLIE),
					challenger_play.0,
					challenger_play.1,
					rival_play.0,
					rival_play.1,
					challenge_id
				),
				Error::<Test>::CannotRevealNonParticipatingChallenge
			);
		});
	}

	#[test]
	fn should_fail_to_revolve_non_fully_played_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let rival = BOB;

			let challenge_id = create_accepted_challenge(challenger, rival);

			assert_ok!(
				RpsModule::play_challenge(
					Origin::signed(challenger),
					challenge_id,
					challenger_play.0,
					challenger_play.1
				),
				()
			);

			assert_noop!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					challenger_play.0,
					challenger_play.1,
					challenge_id
				),
				Error::<Test>::ChallengeStateForbidsResolution
			);
		});
	}

	#[test]
	fn should_fail_to_resolve_non_accepted_challenge() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let challenge_id = create_challenge(challenger);

			assert_noop!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					challenger_play.0,
					challenger_play.1,
					challenge_id
				),
				Error::<Test>::ChallengeStateForbidsPlay
			);
		});
	}

	#[test]
	fn should_fail_to_resolve_with_incorrect_secret() {
		new_test_ext(&ENDOWED_ACCOUNTS, ENDOWMENT_AMOUNT).execute_with(|| {
			let challenger = ALICE;
			let challenger_play = (ChallengePlay::Scissors, 57832_u64);

			let rival = BOB;
			let rival_play = (ChallengePlay::Rock, 481_u64);

			let challenge_id =
				create_played_challenge(challenger, rival, challenger_play, rival_play);

			assert_noop!(
				RpsModule::reveal_challenge_results(
					Origin::signed(challenger),
					challenger_play.0,
					challenger_play.1,
					rival_play.0,
					rival_play.1 + 12,
					challenge_id
				),
				Error::<Test>::InvalidHandHash
			);
		});
	}
}
