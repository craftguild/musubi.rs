// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

/// @dev External compiler/reference harness for lab02 EIP-712 edge vectors.
contract TypedDataEdgeReferenceTest {
    bytes32 private constant EDGE_TYPEHASH = 0x9725f5cb8d96d8246decae61272937776dcef90446bf67e742cd941d7d875985;
    bytes32 private constant EDGE_STRUCT_HASH = 0xbcb950b91075be76a296e92895a1940cc3d691988906eac237a7c19e546ef545;

    function testEdgeVector() public pure {
        uint16[2] memory weights = [uint16(0), type(uint16).max];
        bytes32 typeHash = keccak256(bytes("Edge(int24 tick,bytes4 tag,uint16[2] weights)"));
        bytes32 arrayHash = keccak256(abi.encode(weights[0], weights[1]));
        bytes32 structHash = keccak256(abi.encode(typeHash, type(int24).min, bytes4(0xdeadbeef), arrayHash));

        require(typeHash == EDGE_TYPEHASH, "edge type hash");
        require(structHash == EDGE_STRUCT_HASH, "edge struct hash");
    }
}
