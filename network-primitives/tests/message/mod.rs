use beserial::{Deserialize, Serialize};
use nimiq_network_primitives::message::*;

const VERSION_MESSAGE: &str = "42042042000000010ee4e19ae300000001040000000400000167aaa7c40d02a84eaf654fe5f3b0bb45d0dd9a70c78fc24d134f5e302aa8270ea107752a6b860053e4c4966637a7de44500e8df82d7b541f578ab25a9e147fed9066361081826337f5511fa27762ecd0e328488e48bcbc4c6e2ded7b552039832768e4f137d809096c6f63616c686f737420fb264aaf8a4f9828a76c550635da078eb466306a189fcc03710bee9f649c869d12c6efcae1d34d135ff562bd75a62ffbcaab81f578ad23da8a02ccf59c7f8b6baa97fabe9dbd9db0acb5e1539bf3155ca1c9565f3363c5c8f1e1cc5b99ba3902c921636f72652d6a732f312e342e3120286e6f64656a733b204c696e75782078363429";
const INV_MESSAGE: &str = "42042042010000007b268c0610000300000002324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf00000002b8b37c1d034e371c7a3b834f9476a746eb62259ff9558ab715b4bff79ebf58e100000001f823f66ba1026e7f711ea5aa4719837bb378fc615b50516b8dabdaff78e8168e";
const GET_DATA_MESSAGE: &str = "42042042020000007b990afcc2000300000002324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf00000002b8b37c1d034e371c7a3b834f9476a746eb62259ff9558ab715b4bff79ebf58e100000001f823f66ba1026e7f711ea5aa4719837bb378fc615b50516b8dabdaff78e8168e";
const BLOCK_MESSAGE: &str = "4204204206000000ba2002774c0001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009d5b7130fdd19427406f1d477788ec1f866c650b2eff550afb3505b1435681bbbfacd81c643767f3bf1bf642c97b4efd33daeff2b0d341c613344ae706eedb381f010000000000010000000000018d5800011be440919634a6fe3ba5f8a7181fe4bb8212c13c0000000000";
const HEADER_MESSAGE: &str = "42042042070000009fc8a9c2ed0001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009d5b7130fdd19427406f1d477788ec1f866c650b2eff550afb3505b1435681bbbfacd81c643767f3bf1bf642c97b4efd33daeff2b0d341c613344ae706eedb381f010000000000010000000000018d58";
const TX_MESSAGE: &str = "42042042080000009803ad3340008f30a4d938d4130d1a3396dede1505c72b7f75ac9f9b80d1ad7e368b39f3b10591e9240f415223982edc345532630710e94a7f5200000000000022b8000000000000002a0000000004e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b00";
const GET_BLOCKS_MESSAGE: &str = "420420420500000012f392d9600000000402";
const MEMPOOL_MESSAGE: &str = "42042042090000000d994373bd";
const REJECT_MESSAGE: &str = "420420420a000000194422360c004104746573740003616263";
const ADDR_MESSAGE: &str = "4204204214000000f5650e831a00020000000000000000000000000000080808088f30a4d938d4130d1a3396dede1505c72b7f75ac9f9b80d1ad7e368b39f3b10500e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b0000000000000000000000000000080808088f30a4d938d4130d1a3396dede1505c72b7f75ac9f9b80d1ad7e368b39f3b10500e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
const GET_ADDR_MESSAGE: &str = "420420421500000014c09a093a02000000040008";
const PING_MESSAGE: &str = "420420421600000011fde10bd200000002";
const PONG_MESSAGE: &str = "4204204217000000112077d25700000002";
const SUBSCRIBE_MESSAGE: &str = "420420420b000000601dc2dcab02000491e9240f415223982edc345532630710e94a7f5287298cc2f31fba73181ea2a9e6ef10dce21ed95e47ea70cf08872bdb4afad3432b01d963ac7d165f5d1c3122ada85138a67dfc15267cbeb21dd36041";
const GET_CHAIN_PROOF_MESSAGE: &str = "42042042280000000d0cc9e55d";
const GET_ACCOUNTS_PROOF_MESSAGE: &str = "420420422a000000570169b3b4324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf000291e9240f415223982edc345532630710e94a7f5287298cc2f31fba73181ea2a9e6ef10dce21ed95e";
const ACCOUNTS_PROOF_MESSAGE_WPROOF: &str = "420420422b00000269694947d7324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf010007ff2830303131313131313131313131313131313131313131313131313131313131313131313131313131000000000000000019ff283030323030303030303030303030303030303030303030303030303030303030303030303030303000000000000000052aff283030323232323232323232323232323232323232323232323232323232323232323232323232323200000000000000005d00033030320225303030303030303030303030303030303030303030303030303030303030303030303030304f4c455662cdbcb30bcc2719dc7184cc30aa01ece81e5a8070f0a2e2f37f5f6b2532323232323232323232323232323232323232323232323232323232323232323232323232d2d93f95e11fc480f2cddb6b3a38e457e24bce6df5c5f470b2501acd5d61e12bff283030333333333333333333333333333333333333333333333333333333333333333333333333333300000000000000000100023030032631313131313131313131313131313131313131313131313131313131313131313131313131314a222c2f95a2f733d51ddea9ea7f6f0bc8f66f7ff4a3c7cc26ae99c4dcc5e28a0132b4e74f0d8054d535e1cb108fab3772a4e19729777b51d4640665a2da6ec7588e263333333333333333333333333333333333333333333333333333333333333333333333333333516c248e6df72f9ffdcc6be8bda172077a8f2de2956bf54d21cffdb77d4c8b02000001023030de17952823fba92b774b125dd808c270dccf1fe3b9c80b1050516ece11c787bb";
const ACCOUNTS_PROOF_MESSAGE_WOPROOF: &str = "420420422b0000002e3fb4ef97324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf00";
const GET_ACCOUNTS_TREE_CHUNK_MESSAGE: &str = "420420422c00000030078e5b06324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf026161";
const GET_TRANSACTIONS_PROOF_MESSAGE: &str = "420420422f000000571ee934b2324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cf000291e9240f415223982edc345532630710e94a7f5287298cc2f31fba73181ea2a9e6ef10dce21ed95e";
const GET_TRANSACTION_RECEIPTS_MESSAGE: &str = "42042042310000002525dc96f691e9240f415223982edc345532630710e94a7f5200000000";
const GET_BLOCK_PROOF_MESSAGE: &str = "42042042330000004d3f619793324dcf027dd4a30a932c441f365a25e86b173defa4b8e58948253471b81b72cfb8b37c1d034e371c7a3b834f9476a746eb62259ff9558ab715b4bff79ebf58e1";


static MESSAGES: &'static [&str] = &[
    VERSION_MESSAGE,
    INV_MESSAGE,
    GET_DATA_MESSAGE,
    BLOCK_MESSAGE,
    HEADER_MESSAGE,
    TX_MESSAGE,
    GET_BLOCKS_MESSAGE,
    MEMPOOL_MESSAGE,
    REJECT_MESSAGE,
    ADDR_MESSAGE,
    GET_ADDR_MESSAGE,
    PING_MESSAGE,
    PONG_MESSAGE,
//    SUBSCRIBE_MESSAGE, // FIXME: HashSets don't preserve insertion order and addresses on SubscribeMsg are stored in a HashSet
    GET_CHAIN_PROOF_MESSAGE,
//    ACCOUNTS_PROOF_MESSAGE_WPROOF,
//    ACCOUNTS_PROOF_MESSAGE_WOPROOF,
    GET_ACCOUNTS_TREE_CHUNK_MESSAGE,
    GET_TRANSACTIONS_PROOF_MESSAGE,
    GET_TRANSACTION_RECEIPTS_MESSAGE,
    GET_BLOCK_PROOF_MESSAGE,
//    BLOCK_PROOF_MESSAGE
];

#[test]
fn parse_version_message() {
    let vec = ::hex::decode(VERSION_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Version(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_inv_message() {
    let vec = ::hex::decode(INV_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Inv(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_data_message() {
    let vec = ::hex::decode(GET_DATA_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetData(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_block_message() {
    let vec = ::hex::decode(BLOCK_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Block(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_header_message() {
    let vec = ::hex::decode(HEADER_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Header(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_tx_message() {
    let vec = ::hex::decode(TX_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Tx(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_blocks_message() {
    let vec = ::hex::decode(GET_BLOCKS_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetBlocks(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_mempool_message() {
    let vec = ::hex::decode(MEMPOOL_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Mempool => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_reject_message() {
    let vec = ::hex::decode(REJECT_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Reject(_) => assert!(true), _ => assert!(false) };
}

//#[test]
//fn parse_subscribe_message() {
//    let hex_msg = "420420420b000000382a2e67c102000291e9240f415223982edc345532630710e94a7f5287298cc2f31fba73181ea2a9e6ef10dce21ed95e";
//    let vec = ::hex::decode(hex_msg).unwrap();
//    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
//    match message { Message::Subscribe(_) => assert!(true), _ => assert!(false) };
//}

#[test]
fn parse_addr_message() {
    let vec = ::hex::decode(ADDR_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Addr(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_addr_message() {
    let vec = ::hex::decode(GET_ADDR_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetAddr(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_ping_message() {
    let vec = ::hex::decode(PING_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Ping(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_pong_message() {
    let vec = ::hex::decode(PONG_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Pong(_) => assert!(true), _ => assert!(false) };
}


#[test]
fn parse_subscribe_message() {
    let vec = ::hex::decode(SUBSCRIBE_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::Subscribe(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_chain_proof_message() {
    let vec = ::hex::decode(GET_CHAIN_PROOF_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetChainProof => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_accounts_proof_message() {
    let vec = ::hex::decode(GET_ACCOUNTS_PROOF_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message {
        Message::GetAccountsProof(get_accounts_proof_message) => {
            assert!(get_accounts_proof_message.addresses.len() == 2);
        },
        _ => assert!(false)
    };
}

#[test]
fn parse_accounts_proof_message_wproof() {
    let vec = ::hex::decode(ACCOUNTS_PROOF_MESSAGE_WPROOF).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message {
        Message::AccountsProof(accounts_proof_message) => {
            assert!(accounts_proof_message.proof.is_some());
        },
        _ => assert!(false)
    };
}

#[test]
fn parse_accounts_proof_message_woproof() {
    let vec = ::hex::decode(ACCOUNTS_PROOF_MESSAGE_WOPROOF).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message {
        Message::AccountsProof(accounts_proof_message) => {
            assert!(accounts_proof_message.proof.is_none());
        },
        _ => assert!(false)
    };
}

#[test]
fn parse_accounts_tree_chunk_message_woproof() {
    let vec = ::hex::decode(GET_ACCOUNTS_TREE_CHUNK_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message {
        Message::GetAccountsTreeChunk(get_accounts_tree_chunk_message) => {
            assert!(get_accounts_tree_chunk_message.start_prefix == "aa");
        },
        _ => assert!(false)
    };
}


//fn parse_accounts_tree_chunk_message() {
    //TODO
//}

#[test]
fn parse_get_transactions_proof_message() {
    let vec = ::hex::decode(GET_TRANSACTIONS_PROOF_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message {
        Message::GetTransactionsProof(get_transactions_proof_message) => {
            assert!(get_transactions_proof_message.addresses.len() == 2);
        },
        _ => assert!(false)
    };
}

#[test]
fn parse_get_transaction_receipts_message() {
    let vec = ::hex::decode(GET_TRANSACTION_RECEIPTS_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetTransactionReceipts(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn parse_get_block_proof() {
    let vec = ::hex::decode(GET_BLOCK_PROOF_MESSAGE).unwrap();
    let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
    match message { Message::GetBlockProof(_) => assert!(true), _ => assert!(false) };
}

#[test]
fn reserialize_messages() {
    for message in MESSAGES.iter() {
        let vec = ::hex::decode(message).unwrap();
        let message: Message = Deserialize::deserialize(&mut &vec[..]).unwrap();
        assert!(message.serialize_to_vec() == vec);
    }
}
