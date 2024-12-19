// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.14;

import { Test} from "forge-std/Test.sol";
import { console } from "forge-std/console.sol";
import { UniswapV3Simulator } from "../src/UniswapV3Simulator.sol";
import { IERC20 } from "../src/Interfaces.sol";

contract UniswapV3SimulatorTest is Test {
    IERC20 WETH = IERC20(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
    IERC20 USDT = IERC20(0xdAC17F958D2ee523a2206206994597C13D831ec7);
    address POOL = address(0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36);
    
    string RPC_URL = "https://eth-mainnet.g.alchemy.com/v2/Sg0Hh6Bcv4Dfj2OcU4_6VePVPED-8-MD";
    
    function setUp() public { }

    function testGetBytecode() public {
        UniswapV3Simulator simulator = new UniswapV3Simulator();
        console.logBytes(address(simulator).code);
    }

    function testSimpleSwap() public {
        bool zeroForOne = true; // `true` indicates that we want to swap token0 for token1.
        uint256 amountIn = 1 ether; // Positive indicates we are swapping exact input, negative indicates exact output.

        UniswapV3Simulator simulator = new UniswapV3Simulator();

        vm.selectFork(vm.createFork(RPC_URL, 21424541));

        deal(address(WETH), address(simulator), 10 ether);

        uint256 tokenInBeforeTest = WETH.balanceOf(address(simulator));
        uint256 tokenOutBeforeTest = USDT.balanceOf(address(simulator));

        (uint256 tokenInBalanceBefore, uint256 tokenInBalanceAfter, uint256 tokenOutBalanceBefore, uint256 tokenOutBalanceAfter) = 
            simulator.swap(POOL, address(WETH), address(USDT), zeroForOne, amountIn);

        uint256 tokenInAfterTest = WETH.balanceOf(address(simulator));
        uint256 tokenOutAfterTest = USDT.balanceOf(address(simulator));

        assertEq(tokenInBalanceBefore, tokenInBeforeTest);
        assertEq(tokenOutBalanceBefore, tokenOutBeforeTest);
        assertEq(tokenInBalanceAfter, tokenInAfterTest);
        assertEq(tokenOutBalanceAfter, tokenOutAfterTest);

        assertEq(tokenInBalanceAfter, tokenInBalanceBefore - amountIn);
    }
}
