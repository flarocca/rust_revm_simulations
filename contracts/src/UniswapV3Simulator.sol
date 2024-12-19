// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.14;

import { console } from "forge-std/console.sol";
import "./HelperLibraries.sol";
import { IERC20, IUniswapV3Pool } from "./Interfaces.sol";

contract UniswapV3Simulator {
    uint160 internal constant MIN_SQRT_RATIO = 4295128739;
    uint160 internal constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;

    function getPoolData(address pool)
        external
        view
        returns (
            address token00,
            address token01,
            address factory,
            uint24 fee
        )
    {
        
        token00 = IUniswapV3Pool(pool).token0();
        token01 = IUniswapV3Pool(pool).token1();
        factory = IUniswapV3Pool(pool).factory();
        fee = IUniswapV3Pool(pool).fee();
    }

    function swap(
        address poolAddress,
        address recipient,
        address tokenIn,
        address tokenOut,
        bool zeroForOne,
        uint256 amountIn
    )
        external
        returns (
            uint256 tokenInBalanceBefore,
            uint256 tokenInBalanceAfter,
            uint256 tokenOutBalanceBefore,
            uint256 tokenOutBalanceAfter
        )
    {
        tokenInBalanceBefore = IERC20(tokenIn).balanceOf(recipient);
        tokenOutBalanceBefore = IERC20(tokenOut).balanceOf(recipient);

        // We ignore the return values to rely on the real balances anfter swapping.
        // This is because some tokens may have a fee on transfer, which will affect the balances.
        internal_swap(recipient, poolAddress, tokenIn, zeroForOne, int256(amountIn));

        tokenInBalanceAfter = IERC20(tokenIn).balanceOf(recipient);
        tokenOutBalanceAfter = IERC20(tokenOut).balanceOf(recipient);
    }

    function uniswapV3SwapCallback(int256 amount0Out, int256 amount1Out, bytes calldata data) external {
        bytes memory dataBytes = data;
        address tokenOut;
        assembly {
            tokenOut := mload(add(dataBytes, 20))
        }

        if (amount0Out > 0) {
            IERC20(tokenOut).transfer(msg.sender, uint256(amount0Out));
        } else if (amount1Out > 0) {
            IERC20(tokenOut).transfer(msg.sender, uint256(amount1Out));
        }
    }

    function internal_swap(address recipient, address poolAddress, address tokenIn, bool zeroForOne, int256 amount)
        internal
        returns (int256, int256)
    {
        bytes memory addressBytes = abi.encodePacked(tokenIn);

        try IUniswapV3Pool(poolAddress).swap(
            recipient, zeroForOne, amount, (zeroForOne ? MIN_SQRT_RATIO + 1 : MAX_SQRT_RATIO - 1), addressBytes
        ) returns (int256 amount0Delta, int256 amount1Delta) {
            return (amount0Delta, amount1Delta);
        } catch (bytes memory lowLevelData) {
            revert(string(abi.encodePacked("UNISWAP_V3 Revert: ", lowLevelData)));
        }
    }
}
