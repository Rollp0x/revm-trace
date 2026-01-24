use alloy::sol;

// https://eips.ethereum.org/EIPS/eip-721
sol! {
    #[sol(rpc)]
    contract IERC721 {
        event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
        event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId);
        event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

        function balanceOf(address owner) external view returns (uint256);
        function ownerOf(uint256 tokenId) external view returns (address);
        function safeTransferFrom(address from, address to, uint256 tokenId, bytes data) external payable;
        function safeTransferFrom(address from, address to, uint256 tokenId) external payable;
        function transferFrom(address from, address to, uint256 tokenId) external payable;
        function approve(address approved, uint256 tokenId) external payable;
        function setApprovalForAll(address operator, bool approved) external;
        function getApproved(uint256 tokenId) external view returns (address);
        function isApprovedForAll(address owner, address operator) external view returns (bool);

        // metadata
        function name() external view returns (string name);
        function symbol() external view returns (string symbol);
        function tokenURI(uint256 tokenId) external view returns (string);
    }
}
