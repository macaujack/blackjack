# Blackjack

A Blackjack program to help you make decisions in Blackjack.

## TODO
- Improve Simulator to support full simulation of Blackjack.
- Improve calculation of solution.
    - ~~Bugfix: Double can only be performed on initial hands.~~
    - Calculate the winning EX of a given shoe in the betting phase (i.e., decide whether to place bets this turn).
    - (Really necessary when no Split?) Speed calculation by multi-threading.
    - Take Split into consideration.
    - (Low priority) Predict card dealt to others, and make card count floating point numbers, instead of integers.
- Implement a driver to run Blackjack simulation.
- Implement a blackjack helper to assist user to play Blackjack.
- (Low priority) Improve Basic Strategy by calculating instead of hard-coding.
