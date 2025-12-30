# API

## Requesting a new game

Make a request to `/new` with the following data

```json
{
  "email": "your-email@domain.com"
}
```

The response is in the format:

```json
{
  "status": "success",
  data: {
    "token": "<jwt-token>"
  }
}
```

## Getting the game data

All subsequent requests expect `Authorization: Bearer <jwt>` token in the header.

Make a request to `/status` to retrieve the current state of the game.
The response is in the format.

```json
{
  "status": "success",
  "data": {
    "players": ["player1", "player2", "you", "player3"],
    "rounds": [
      "calls": ["3", "2", "2", "3"],
      "hand": [{"rank": "ace", "suit": "hearts"}, {"rank": "two", "suit": "spades"), ...],
      "tricks": [
        [{"rank": "jack", "suit": "clubs"}, ...],
        ...
      ],
      "calls": ["", "2", "", ""],
      "hand": [...],
      "tricks": [
        []
      ]
    ]
  }
}
```

## Make a call

Make a request to `/call` with the following data:

```json
{"call": "value"}
```

The response will be in the format:

```json
{
  "status": "success",
  "data": {
    "<same format as game state>"
  }
}
```

## Make a break

Make a request to `/break` with the following data:

```json
{
  "rank": "ace", 
  "suit": "spades"
}
```

The response will be in the format:

```json
{
  "status": "success",
  "data": {
    "<same format as game state>"
  }
}
```

## Request valid move

Make a request to `/validmove`. The response will be in the following format:

```json
{
  "status": "success",
  "data": {
    "action": "call/break",
    "values": "in case of break, this will return an array of cards that can be played"
  }
}
```

TODO: it might be nice to just append this to the `/status` at the end.
Or to the general game state which will make playing the game for bots a lot easier.
