// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

interface VmLogRecorder {
    struct Log {
        bytes32[] topics;
        bytes data;
        address emitter;
    }

    function recordLogs() external;
    function getRecordedLogs() external returns (Log[] memory);
}

/// @dev External emitted-log reference harness for the lab02 generated event codec.
contract RebalanceEventReferenceTest {
    VmLogRecorder private constant VM = VmLogRecorder(address(uint160(uint256(keccak256("hevm cheat code")))));
    bytes32 private constant POLICY_HASH = hex"1111111111111111111111111111111111111111111111111111111111111111";
    bytes32 private constant POOL_HASH = hex"2222222222222222222222222222222222222222222222222222222222222222";
    bytes32 private constant PLAN_HASH = hex"3333333333333333333333333333333333333333333333333333333333333333";
    bytes32 private constant INTENT_HASH = hex"4444444444444444444444444444444444444444444444444444444444444444";

    event RebalanceExecuted(
        bytes32 indexed policyHash,
        uint256 indexed executionNonce,
        bytes32 indexed poolConfigHash,
        bytes32 planHash,
        bytes32 intentHash,
        uint256 oldPositionIdentifier,
        uint256 newPositionIdentifier,
        int24 tickLower,
        int24 tickUpper,
        uint256 swapAmount0In,
        uint256 swapAmount1In,
        uint256 swapAmount0Out,
        uint256 swapAmount1Out,
        uint256 principal0Collected,
        uint256 principal1Collected,
        uint256 fee0Collected,
        uint256 fee1Collected,
        uint256 amount0Deposited,
        uint256 amount1Deposited,
        uint256 idle0After,
        uint256 idle1After,
        uint128 liquidityAdded
    );

    function testRebalanceExecutedLog() public {
        VM.recordLogs();
        emit RebalanceExecuted(
            POLICY_HASH, 7, POOL_HASH, PLAN_HASH, INTENT_HASH, 1, 2, -2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
        );
        VmLogRecorder.Log[] memory logs = VM.getRecordedLogs();
        require(logs.length == 1, "log count");
        require(logs[0].topics.length == 4, "topic count");
        require(
            logs[0].topics[0]
                == keccak256(
                    "RebalanceExecuted(bytes32,uint256,bytes32,bytes32,bytes32,uint256,uint256,int24,int24,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint128)"
                ),
            "signature topic"
        );
        require(logs[0].topics[1] == POLICY_HASH, "policy topic");
        require(logs[0].topics[2] == bytes32(uint256(7)), "nonce topic");
        require(logs[0].topics[3] == POOL_HASH, "pool topic");
        bytes memory expected = abi.encode(
            PLAN_HASH,
            INTENT_HASH,
            uint256(1),
            uint256(2),
            int24(-2),
            int24(3),
            uint256(4),
            uint256(5),
            uint256(6),
            uint256(7),
            uint256(8),
            uint256(9),
            uint256(10),
            uint256(11),
            uint256(12),
            uint256(13),
            uint256(14),
            uint256(15),
            uint128(16)
        );
        require(keccak256(logs[0].data) == keccak256(expected), "event data");
    }
}
