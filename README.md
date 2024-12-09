# Uniswap Swaps tricks

This repository is intended to be educational and to show some tricks that can be done with Uniswap swaps in order to reduce the amount of gas used.

## Router vs Pools

The operational unit for Uniswap are pools, which hold the liquidity for each pair of tokens. The challenge when swapping tokens is to find the pool for the pair
we want to swap and also to compute the amounts, as pools require specific inputs or outputs. Since the pool does not refund remanent tokens, sending more inputs
tokens or asking for less implies a loss of funds. On the other hand asking for more than it should be implies that the swap will fail.
The Router is the one that, apart from holding all available pools, takes care of computing the correct amounts and interfacing with the pool.
The disadvantage of using the Router is that it is more expensive in terms of gas, as it has to do execute more `opcodes`.

## Simulating with the Router, but swapping with the pool

Leveraging `revm`, we can simulate the swap using the router and discover the underlaying pool used and the corresponding amounts.
Once we discovered the pool and the inputs, we can simply execute the swap via the pool directly, saving gas.

## Discovering the pool

When a swap is performed, pools emit Swap events. We can extract the emitted events from the transaction, filter out the Swap events and extract the pool address
along with the amounts.

## Router operation

When execiuting a swap via the Router, we need to ensure we allow the router to transfer the input token on out behalf.
This is because the Router call `transferFrom` on the input token using the caller as `source` and the pool as `destination`.

## Pool operation

When executing a swap via the pool, we first need to transfer the input token amount and then call `swap` on the pool specifying the exact output token amount,
and also whether we want `token1` or `token0`. That information is extracted from the Swap event emited when simulated the swap via the Router.

```rust
// Pseudo-code example

// Step 1: Simulate the swap via the Router
token.approve(router, amount);
let (pool, amount0_out, amount1_out) = router.swap_exact_tokens_for_tokens(
    amount,
    amount_out_min,
    [token_in, token_out],
    caller,
    deadline,
);

// Step 2: Simulate the swap via the Pool
token.transfer(pool, amount);
pool.swap(
    amount0_out,
    amount1_out,
    caller,
);
```

## Why the router?

From the example above, we can see that the pool expects exact output values from each token in the pool.
As mentioned before, sending more input tokens than the required does not fail but are not refunded.
On the other hand, asking for more than the expectation results in a IIA failure.

We can do the math by hand if we want, but that implies querying the current reserves along token0 and token1.
And we also need to know the pool we want to swap with.

By using the Router, we can get all that information securely and in a single call.

