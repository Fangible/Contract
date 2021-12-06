use solana_client::rpc_request::TokenAccountsFilter;
use dotenv::dotenv;
use std::env;
use spl_associated_token_account::{ create_associated_token_account };
use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    metaplex_token_metadata::{
        instruction::{
            create_master_edition, create_metadata_accounts,
            mint_new_edition_from_master_edition_via_token, puff_metadata_account,
            update_metadata_accounts,
        },
        state::{
            get_reservation_list, Data, Edition, Key, MasterEditionV1, MasterEditionV2, Metadata,
            EDITION, MAX_NAME_LENGTH, MAX_SYMBOL_LENGTH, MAX_URI_LENGTH, PREFIX, Creator,
        },
    },
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{
        account_info::AccountInfo, borsh::try_from_slice_unchecked, program_pack::Pack,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{initialize_account, initialize_mint, mint_to},
        state::{Account, Mint},
    },
    std::str::FromStr,
};

const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const secretkey: &str = "";


fn main() {
    
    println!("Here We Go!");
    let payer = Keypair::from_base58_string(secretkey);
    dotenv().ok();
    let mut instructions = vec![];
    let new_mint = Keypair::new();
    let client = RpcClient::new("https://api.devnet.solana.com".to_string());    

    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let metadata_program = Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap();
    let new_mint_pubkey = &new_mint.pubkey();
    let metadata_seeds = &[PREFIX.as_bytes(), &metadata_program.as_ref(), new_mint_pubkey.as_ref()];
    let (metadata_key, _) = Pubkey::find_program_address(metadata_seeds, &metadata_program);
    //创建mint
    let mut new_mint_instructions = vec![
        create_account(
            &payer.pubkey(),
            &new_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
        initialize_mint(
            &token_key,
            &new_mint.pubkey(),
            &payer.pubkey(),
            Some(&payer.pubkey()),
            0,
        )
        .unwrap(),
    ];
    
    let new_token_account_instruction = create_associated_token_account(
        &payer.pubkey(),
        &payer.pubkey(),
        &new_mint_pubkey,
    );

    let creator = Creator{address:payer.pubkey(),verified:true,share:100};
    //创建metada
    let new_metadata_instruction = create_metadata_accounts(
        metadata_program, //program 
        metadata_key,
        new_mint.pubkey(),
        payer.pubkey(),
        payer.pubkey(),
        payer.pubkey(),
        "The Monster".to_string(),
        "MONST".to_string(),
        "https://ap1-cfs3-media-bounce.bounce.finance/46d7e9bb78997a4a1fb49250ea506a84-1636964700.json".to_string(),
        Some(vec![creator]),
        100,
        true,
        true,
    );

    instructions.append(&mut new_mint_instructions);
    instructions.push(new_token_account_instruction);
    instructions.push(new_metadata_instruction);
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let mut signers = vec![&payer];
    signers.push(&new_mint);
    signers.push(&payer);
    transaction.sign(&signers, recent_blockhash);
    let signature = client.send_and_confirm_transaction(&transaction).unwrap();
    let account = client.get_account(&metadata_key).unwrap();
    let metadata: Metadata = try_from_slice_unchecked(&account.data).unwrap();
    println!("signature:::{:?}",&signature);
    println!("metadata_key:::{:?}",&metadata_key);
}
