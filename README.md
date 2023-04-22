# Blackjack

A Blackjack program to help you make decisions in Blackjack.

## TODO
- Improve Simulator to support full simulation of Blackjack.
- Improve calculation of solution.
    - Make card count floating point numbers, instead of integers.
    - Take Split into consideration.
    - (Possible?) Calculate the winning EX of a given shoe in the betting phase (i.e., decide whether to place bets this turn).
    - Speed up the No-Split calculation by multi-threading. The calculation of ex_stand can be dispatched to multiple threads.
- Implement a driver to run Blackjack simulation.
- Implement a blackjack helper to assist user to play Blackjack.
- (Low priority) Improve Basic Strategy by calculating instead of hard-coding.
