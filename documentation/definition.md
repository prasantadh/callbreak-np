# Game Sequence and Terminology

This document explains the terminology as used in this project and how it corresponds to the gameplay. It also describes the structure and sequence of events in  Callbreak card game, as implemented in this project. 

---

## Overview of the Game

Callbreak is a trick-taking card game played over 5 smaller games (referred to as "rounds"). Each round consists of 13 tricks, where players compete to win as many tricks as they predict at the start of the round. Tricks is the "smallest unit of gameplay," where each player plays one card in turn.

- **Number of players**: 4
- **Number of rounds**: 5
- **Cards per player per round**: 13
- **Tricks per round**: 13
- **Turns per trick**: 4 (one per player)

### Objective

The objective of the game is for players to:
1. Predict how many tricks they will win in a round (**call**).
2. Play cards strategically to achieve or exceed their prediction.
3. Score based on their performance across all five rounds.

---

## Key Events and Terminology

### **Game**
- **Definition**: The overarching match that includes 5 "rounds."
- **Start**: When players are added to the game and ready to play.
- **End**: After all 5 rounds are completed, scores are totaled, and a winner is declared.
- **Code Reference**: `Game` struct in `game.rs`.

### **Round**
- **Definition**: A single game session within the match, consisting of 13 tricks.
  - Involves dealing cards, making **calls**, and playing tricks.
- **Start**: Cards are dealt (with the dealer position rotating clockwise each round), and players declare the number of tricks they aim to win.
- **End**: After all 13 tricks are played, scores are calculated. Players who meet or exceed their call get postiive points; otherwise, they receive negative points.
- **Code Reference**: `Round` struct in `round.rs`.

### **Trick**
- **Definition**: A single sequence of card plays, where each player plays one card. There is a winner for each trick based on the rules of suit and rank.
- **Start**:The trick leader plays a card. For the first trick, the leader is the player to the right of the dealer; for subsequent tricks, the leader is the winner of the previous trick.
- **End**: After all four players have played one card in clockwise order from the leader, the trick is resolved and a winner is determined.
- **Code Reference**: `Trick` struct in `trick.rs`.

### **Call**
- **Definition**: A declaration made at the beginning of a round, where each player predicts how many tricks they aim to win during that round.
- **Purpose**: Calls influence scoring at the end of the round, rewarding players for accuracy.
- **Code Reference**: `Call` struct in `call.rs`.

### **Turn**
- **Definition**: A single player's action during a trick, where they play one card.
- **Order**: Follows a clockwise rotation, starting with the trick leader.
- **Code Reference**: `Turn` struct in `turn.rs`.

### **Hand**
- **Definition**: The set of 13 cards dealt to each player at the start of a round.
- **Depletion**: Players gradually play cards from their hands during tricks.
- **Code Reference**: `Hand` struct in `hand.rs`.

---

## Sequence of Events

Here's a breakdown of the game's sequence, from start to finish:

1. **Start of the Match**:
   - Players join the game.
   - The game begins and transitions out of the Lobby state.

2. **Start of Each Round**:
   - Cards are shuffled and dealt to players.
   - Each player makes their **call**, predicting the number of tricks they expect to win.

3. **Start of Each Trick**:
   - Players take their **turns** (one card played per player).
   - All four played cards are evaluated to determine the winner of the trick.

4. **End of Round**:
   - After 13 tricks, the round ends.
   - Scores are calculated based on the matches between calls and tricks won.
   - The game either ends (after 5 rounds) or moves to the next round.

5. **End of Game**:
   - After all 5 rounds have been played, the match ends.
   - The player with the highest cumulative score is declared the winner.

---

## Terminology Summary Table

| Term    | Description                                | Code Reference         |
|---------|--------------------------------------------|------------------------|
| **Game**   | The entire match (5 rounds).              | `Game` struct (`game.rs`) |
| **Round**  | One session of 13 tricks.                | `Round` struct (`round.rs`) |
| **Trick**  | Four cards played (one per player).       | `Trick` struct (`trick.rs`) |
| **Call**   | Prediction of tricks to win.             | `Call` struct (`call.rs`) |
| **Turn**   | A single player's action in a trick.     | `Turn` struct (`turn.rs`) |
| **Hand**   | The 13 cards dealt to a player.          | `Hand` struct (`hand.rs`) |

---