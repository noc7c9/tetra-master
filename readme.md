# Tetra Master

Tetra Master is a card-based minigame in Final Fantasy 9.

This is a (mostly) faithful implementation of that game modified to be (mostly)
perfect information game.

![Screenshot](screenshot.png)

# Rules

When run both players will be given a random hand of 5 cards.
The board will also be setup with up to 6 blocked.

-   The players take turns placing cards on the field.
-   If the placed card points to an opponent's card that doesn't point back,
    that card will flipped and belong to the other player.
-   If the opponent's card does point back, the two cards will battle and the
    losing card will be flipped.
-   Also when a card loses a battle, all the cards pointed to by the losing card
    will also flip. This is called a combo.

The goal is to have the most cards once the last card has been played.

# Running

The program is written in Rust and the recommended way to run it is to install
Rust and use `cargo run --release`.
