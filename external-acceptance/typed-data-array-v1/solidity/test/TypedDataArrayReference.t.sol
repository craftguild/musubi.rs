// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.26;

/// @dev External compiler/reference harness for the lab02 dynamic struct-array vector.
contract TypedDataArrayReferenceTest {
    bytes32 private constant PERSON_TYPEHASH = keccak256("Person(string name,address wallet)");
    bytes32 private constant GROUP_TYPEHASH = keccak256("Group(Person[] members)Person(string name,address wallet)");
    bytes32 private constant GROUP_STRUCT_HASH = 0x6ceef4da3b8605bf55265a94cccf738064cb934b396231e09588e696d49bb357;

    function testGroupVector() public pure {
        bytes32 alice = _personHash("Alice", address(0x0101010101010101010101010101010101010101));
        bytes32 bob = _personHash("Bob", address(0x0202020202020202020202020202020202020202));
        bytes32 membersHash = keccak256(abi.encodePacked(alice, bob));
        bytes32 groupHash = keccak256(abi.encode(GROUP_TYPEHASH, membersHash));

        require(groupHash == GROUP_STRUCT_HASH, "group struct hash");
    }

    function _personHash(string memory name, address wallet) private pure returns (bytes32) {
        return keccak256(abi.encode(PERSON_TYPEHASH, keccak256(bytes(name)), wallet));
    }
}
