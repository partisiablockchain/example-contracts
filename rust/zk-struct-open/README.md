# Zero knowledge: Struct Open

Example contract which opens all secret input and saves it to the contract state.

For each input a computation is run which creates a new secret variable with the same value as the secret input.
Then, that variable is opened.