# Uniswap Swaps tricks

This repository is intended to be educational and to show some tricks that can be done with Uniswap swaps in order to reduce the amount of gas used.

## Router vs Pools

The operational unit for Uniswap are pools, which hold the liquidity for each pair of tokens. The challenge when swapping tokens is to find the pool for the pair
we want to swap and also to compute the amounts, as pools require specific inputs or outputs. Since the pool does not refund remanent tokens, sending more inputs
tokens or asking for less implies a loss of funds. On the other hand asking for more than it should be implies that the swap will fail.
The Router is the smart contract that, apart from holding all available pools, takes care of computing the correct amounts and interfacing with the pool.
The disadvantage of using the Router is that it is more expensive in terms of gas, as it has to do execute more `opcodes`.

## Simulating with the Router, but swapping with the pool

Leveraging `revm`, we can simulate the swap using the router and discover the underlaying pool used and the corresponding amounts.
Once we discovered the pool and the inputs, we can simply execute the swap via the pool directly, saving gas.

## Discovering the pool

When a swap is performed, pools emit Swap events. We can extract the emitted events from the transaction, filter out the Swap events and extract the pool address
along with the amounts.



---
WIP
