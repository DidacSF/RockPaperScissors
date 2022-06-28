#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type ChallengeId = u64;

pub type ChallengePlayHash = [u8; 8];

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct OpenChallenge<AccountId: PartialEq + Clone, Balance> {
	challenger: AccountId,
	bet_amount: Balance,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct AcceptedChallenge<AccountId: PartialEq + Clone, Balance> {
	challenger: AccountId,
	rival: AccountId,
	bet_amount: Balance,
}

impl<AccountId: PartialEq + Clone, Balance> AcceptedChallenge<AccountId, Balance> {
	pub fn from_open(open_challenge: OpenChallenge<AccountId, Balance>, rival: AccountId) -> Self {
		AcceptedChallenge {
			challenger: open_challenge.challenger,
			rival,
			bet_amount: open_challenge.bet_amount,
		}
	}

	pub fn contains_player(&self, player: &AccountId) -> bool {
		self.challenger == *player || self.rival == *player
	}

	pub fn get_rival(&self, player: &AccountId) -> Option<AccountId> {
		if self.challenger == *player {
			Some(self.rival.clone())
		} else if self.rival == *player {
			Some(self.challenger.clone())
		} else {
			None
		}
	}
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct FinishedChallenge<AccountId: PartialEq + Clone, Balance> {
	challenger: AccountId,
	rival: AccountId,
	bet_amount: Balance,
	winner: Option<AccountId>,
}

impl<AccountId: PartialEq + Clone, Balance> FinishedChallenge<AccountId, Balance> {
	pub fn from_accepted(
		accepted_challenge: AcceptedChallenge<AccountId, Balance>,
		winner: Option<AccountId>,
	) -> Self {
		FinishedChallenge {
			challenger: accepted_challenge.challenger,
			rival: accepted_challenge.rival,
			bet_amount: accepted_challenge.bet_amount,
			winner,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum ChallengeState<AccountId: PartialEq + Clone, Balance> {
	Open(OpenChallenge<AccountId, Balance>),
	Accepted(AcceptedChallenge<AccountId, Balance>),
	Finished(FinishedChallenge<AccountId, Balance>),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum ChallengePlay {
	Rock,
	Paper,
	Scissors,
}

impl ChallengePlay {
	fn as_bytes(&self) -> [u8; 1] {
		match self {
			ChallengePlay::Rock => 1_u8.to_ne_bytes(),
			ChallengePlay::Paper => 2_u8.to_ne_bytes(),
			ChallengePlay::Scissors => 3_u8.to_ne_bytes(),
		}
	}

	pub fn generate_hash(&self, secret: u64) -> ChallengePlayHash {
		let mut bytes = sp_std::vec::Vec::new();
		bytes.extend(self.as_bytes());
		bytes.extend(secret.to_ne_bytes());
		sp_io::hashing::twox_64(&bytes)
	}

	pub fn compare_hash_with(&self, secret: u64, other_hash: ChallengePlayHash) -> bool {
		self.generate_hash(secret) == other_hash
	}
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum PlayResult {
	Win,
	Lose,
	Draw,
}

impl ChallengePlay {
	pub fn beats(&self, other: &ChallengePlay) -> PlayResult {
		match self {
			ChallengePlay::Rock => match other {
				ChallengePlay::Rock => PlayResult::Draw,
				ChallengePlay::Paper => PlayResult::Lose,
				ChallengePlay::Scissors => PlayResult::Win,
			},
			ChallengePlay::Paper => match other {
				ChallengePlay::Rock => PlayResult::Win,
				ChallengePlay::Paper => PlayResult::Draw,
				ChallengePlay::Scissors => PlayResult::Lose,
			},
			ChallengePlay::Scissors => match other {
				ChallengePlay::Rock => PlayResult::Lose,
				ChallengePlay::Paper => PlayResult::Win,
				ChallengePlay::Scissors => PlayResult::Draw,
			},
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_std::ops::Mul;

	use frame_support::traits::{BalanceStatus, Currency, ReservableCurrency};
	use frame_system::pallet_prelude::*;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type MinBetAmount: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn next_challenge_id)]
	pub type NextBetId<T> = StorageValue<_, ChallengeId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_store)]
	pub type ChallengeStore<T: Config> =
		StorageMap<_, Blake2_128Concat, ChallengeId, ChallengeState<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_plays_store)]
	pub type ChallengePlaysStore<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChallengeId,
		Blake2_128Concat,
		T::AccountId,
		ChallengePlayHash,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Triggered when a new challenge has been created. [challenge_id, creator_id, bet_amount]
		ChallengeCreated(ChallengeId, T::AccountId, BalanceOf<T>),
		/// Triggered when an account accepts a challenge. [challenge_id, challenger_id]
		EnteredChallenge(ChallengeId, T::AccountId),
		/// Triggered when an account plays in a certain challenge. [challenge_id, challenger_id]
		PlayedInChallenge(ChallengeId, T::AccountId),
		/// Triggered when both players have sent their play on a given challenge. [challenge_id]
		ChallengeReadyForReveal(ChallengeId),
		/// Triggered when a challenge has been finished. [winner_id]
		ChallengeFinished(Option<T::AccountId>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The bet amount is not inferior to the minimum bet value
		InsufficientBetAmount,
		/// The challenge identifier could not be located
		ChallengeNotFound,
		/// The challenge is not open, so it cannot be entered
		ChallengeNotOpen,
		/// Cannot accept challenges created by the same account
		CannotChallengeOneself,
		/// Cannot play in a challenge in which the account is not a participant
		CannotPlayInNonParticipatingChallenge,
		/// Cannot play in a challenge not in the 'Accepted' state
		ChallengeStateForbidsPlay,
		/// Cannot play again in an already played challenge
		ChallengeAlreadyPlayed,
		/// Challenge state is not Accepted so it cannot be resolved
		ChallengeStateForbidsResolution,
		/// The challenge logic reached an invalid state
		InvalidState,
		/// Cannot reveal a challenge in which the account is not a participant
		CannotRevealNonParticipatingChallenge,
		/// The hash of the original account play and the value indicated in the reveal don't match
		InvalidHandHash,
	}

	impl<T> From<DispatchError> for Error<T> {
		fn from(_: DispatchError) -> Self {
			Self::InvalidState
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_challenge(origin: OriginFor<T>, bet_amount: BalanceOf<T>) -> DispatchResult {
			let challenger = ensure_signed(origin)?;

			let min_amount = T::MinBetAmount::get();
			ensure!(bet_amount >= min_amount, Error::<T>::InsufficientBetAmount);

			let challenge_id = NextBetId::<T>::get();
			let challenge_state =
				ChallengeState::Open(OpenChallenge { challenger: challenger.clone(), bet_amount });

			NextBetId::<T>::mutate(|x| *x += 1);

			ChallengeStore::<T>::insert(&challenge_id, challenge_state);

			Self::deposit_event(Event::ChallengeCreated(challenge_id, challenger, bet_amount));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn enter_challenge(origin: OriginFor<T>, challenge_id: ChallengeId) -> DispatchResult {
			let rival = ensure_signed(origin)?;

			Ok(ChallengeStore::<T>::try_mutate(&challenge_id, |challenge_entry| {
				ensure!(challenge_entry.is_some(), Error::<T>::ChallengeNotFound);

				let challenge_state = challenge_entry.as_mut().unwrap();

				if let ChallengeState::Open(open_state) = challenge_state {
					if open_state.challenger == rival {
						Err(Error::<T>::CannotChallengeOneself)
					} else {
						*challenge_state = ChallengeState::Accepted(AcceptedChallenge::from_open(
							open_state.clone(),
							rival.clone(),
						));
						Self::deposit_event(Event::EnteredChallenge(challenge_id, rival));
						Ok(())
					}
				} else {
					Err(Error::<T>::ChallengeNotOpen)
				}
			})?)
		}

		// play
		// - Bet Id
		// - Participant Id
		// - Hand Payload (Encrypted<(HandType (Rock|Paper|Scissor))>, Public-Key)
		#[pallet::weight(10_000)]
		pub fn play_challenge(
			origin: OriginFor<T>,
			challenge_id: ChallengeId,
			challenge_play: ChallengePlay,
			challenger_secret: u64,
		) -> DispatchResult {
			let player = ensure_signed(origin)?;

			let challenge =
				ChallengeStore::<T>::get(&challenge_id).ok_or(Error::<T>::ChallengeNotFound)?;

			if let ChallengeState::Accepted(ref challenge_state) = challenge {
				ensure!(
					challenge_state.contains_player(&player),
					Error::<T>::CannotPlayInNonParticipatingChallenge
				);

				ensure!(
					!ChallengePlaysStore::<T>::contains_key(&challenge_id, &player),
					Error::<T>::CannotPlayInNonParticipatingChallenge
				);

				T::Currency::reserve(&player, challenge_state.bet_amount)?;

				let play_hash = challenge_play.generate_hash(challenger_secret);
				ChallengePlaysStore::<T>::insert(&challenge_id, &player, play_hash);
				Self::deposit_event(Event::PlayedInChallenge(challenge_id, player));

				if ChallengePlaysStore::<T>::iter_key_prefix(&challenge_id).count() == 2 {
					Self::deposit_event(Event::ChallengeReadyForReveal(challenge_id));
				}

				Ok(())
			} else {
				Err(Error::<T>::ChallengeStateForbidsPlay.into())
			}
		}

		// reveal
		// - Challenger Id
		// - Challenger Payload (HandType, Secret)
		#[pallet::weight(10_000)]
		pub fn reveal_challenge_results(
			origin: OriginFor<T>,
			origin_hand: ChallengePlay,
			origin_secret: u64,
			rival_hand: ChallengePlay,
			rival_secret: u64,
			challenge_id: ChallengeId,
		) -> DispatchResult {
			let player = ensure_signed(origin)?;

			Ok(ChallengeStore::<T>::try_mutate(&challenge_id, |challenge_entry| {
				if let Some(challenge) = challenge_entry {
					if let ChallengeState::Accepted(ref challenge_state) = challenge {
						ensure!(
							challenge_state.contains_player(&player),
							Error::<T>::CannotPlayInNonParticipatingChallenge
						);

						let player_hand_hash = Self::get_player_hand_hash(
							&challenge_id,
							&player,
							Error::<T>::ChallengeStateForbidsResolution,
						)?;

						ensure!(
							origin_hand.compare_hash_with(origin_secret, player_hand_hash),
							Error::<T>::InvalidHandHash
						);

						let rival_player =
							challenge_state.get_rival(&player).ok_or(Error::<T>::InvalidState)?;
						let rival_hand_hash = Self::get_player_hand_hash(
							&challenge_id,
							&rival_player,
							Error::<T>::ChallengeStateForbidsResolution,
						)?;
						ensure!(
							rival_hand.compare_hash_with(rival_secret, rival_hand_hash),
							Error::<T>::InvalidHandHash
						);

						let challenge_results = match origin_hand.beats(&rival_hand) {
							PlayResult::Win => Some((&player, &rival_player)),
							PlayResult::Lose => Some((&rival_player, &player)),
							PlayResult::Draw => None,
						};

						if let Some((winner, loser)) = challenge_results {
							T::Currency::repatriate_reserved(
								loser,
								&winner,
								challenge_state.bet_amount,
								BalanceStatus::Reserved,
							)?;
							T::Currency::unreserve(
								winner,
								challenge_state.bet_amount.mul(2_u32.into()),
							);

							*challenge_entry =
								Some(ChallengeState::Finished(FinishedChallenge::from_accepted(
									challenge_state.clone(),
									Some(winner.clone()),
								)));

							Self::deposit_event(Event::ChallengeFinished(Some(winner.clone())));

							Ok(())
						} else {
							T::Currency::unreserve(&player, challenge_state.bet_amount);
							T::Currency::unreserve(&rival_player, challenge_state.bet_amount);

							*challenge_entry = Some(ChallengeState::Finished(
								FinishedChallenge::from_accepted(challenge_state.clone(), None),
							));

							Self::deposit_event(Event::ChallengeFinished(None));

							Ok(())
						}
					} else {
						Err(Error::<T>::ChallengeStateForbidsPlay)
					}
				} else {
					Err(Error::<T>::ChallengeNotFound)
				}
			})?)
		}
	}

	// Internal functions of the pallet
	impl<T: Config> Pallet<T> {
		fn get_player_hand_hash(
			challenge_id: &ChallengeId,
			player_id: &T::AccountId,
			on_error: Error<T>,
		) -> Result<ChallengePlayHash, Error<T>> {
			if let Some(player_hand) = ChallengePlaysStore::<T>::get(&challenge_id, &player_id) {
				Ok(player_hand)
			} else {
				Err(on_error)
			}
		}
	}
}
