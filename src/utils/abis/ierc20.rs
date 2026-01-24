use alloy::sol;

// https://eips.ethereum.org/EIPS/eip-20
sol! {
    #[sol(rpc)]
    contract IERC20 {
        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);

        function name() public view returns (string);
        function symbol() public view returns (string);
        function decimals() public view returns (uint8);
        function totalSupply() public view returns (uint256);
        function balanceOf(address owner) public view returns (uint256 balance);
        function transfer(address to, uint256 value) public returns (bool success);
        function transferFrom(address from, address to, uint256 value) public returns (bool success);
        function approve(address spender, uint256 value) public returns (bool success);
        function allowance(address owner, address spender) public view returns (uint256 remaining);
    }
}
