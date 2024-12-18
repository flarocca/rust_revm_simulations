// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.14;

library SafeMath {
    function add(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require((z = x + y) >= x, "SM_A_OF"); // SAFE_MATH_ADD_OVERFLOW
    }

    function sub(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require((z = x - y) <= x, "SM_S_OF"); // SAFE_MATH_SUB_OVERFLOW
    }

    function mul(uint256 x, uint256 y) internal pure returns (uint256 z) {
        require(y == 0 || (z = x * y) / y == x, "SM_M_OF"); // SAFE_MATH_MUL_OVERFLOW
    }
}

library SafeTransfer {
    function safeTransfer(address token, address to, uint256 value) internal {
        (bool success, bytes memory data) = token.call(abi.encodeWithSelector(0xa9059cbb, to, value));

        require(
            success && (data.length == 0 || abi.decode(data, (bool))), "SF_T_F" // SAFE_TRANSFER_TRANSFER_FAILED
        );
    }
}
