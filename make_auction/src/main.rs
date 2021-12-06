use {
    clap::{crate_description, crate_name, crate_version, App, Arg, ArgMatches, SubCommand},
    solana_clap_utils::{
        input_parsers::pubkey_of,
        input_validators::{is_url, is_valid_pubkey, is_valid_signer},
    },
    solana_client::rpc_client::RpcClient,
    solana_program::{borsh::try_from_slice_unchecked, program_pack::Pack},
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_instruction::create_account,
        transaction::Transaction,
    },
    spl_token::{
        instruction::{approve, initialize_account, initialize_mint, mint_to},
        state::{Account, Mint},
    },
    metaplex_token_vault::{
        instruction::{
            create_activate_vault_instruction, create_add_shares_instruction,
            create_add_token_to_inactive_vault_instruction, create_combine_vault_instruction,
            create_init_vault_instruction, create_mint_shares_instruction,
            create_redeem_shares_instruction, create_update_external_price_account_instruction,
            create_withdraw_shares_instruction, create_withdraw_tokens_instruction,
            create_set_authority_instruction,
        },
        state::{
            ExternalPriceAccount, SafetyDepositBox, Vault, VaultState, MAX_EXTERNAL_ACCOUNT_SIZE,
            MAX_VAULT_SIZE, PREFIX,
        },
    },
    metaplex_auction::{
        instruction::{ create_auction_instruction_v2, set_authority_instruction, start_auction_instruction },
        processor::{
            CancelBidArgs, ClaimBidArgs, CreateAuctionArgs, CreateAuctionArgsV2, EndAuctionArgs,
            PlaceBidArgs, PriceFloor, StartAuctionArgs, WinnerLimit,
        },
    },
    metaplex::{
        instruction::{
            create_init_auction_manager_v2_instruction,
            create_validate_safety_deposit_box_v2_instruction,
            create_start_auction_instruction,
        },
        state::{SafetyDepositConfig, TupleNumericType, WinningConfigType, Key, AmountRange,},
    },
    std::str::FromStr,
};



const TOKEN_PROGRAM_PUBKEY: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SECRET_KEY: &str = "";
const PROGRAM_PUBKEY: &str = "73JKssGRk2zsYEh71yJDn43PtGqJ27V5t3RTzoRwpbu9";
const WSOL: &str = "So11111111111111111111111111111111111111112";
const AUCTION_PROGRAM: &str = "1cuophQB85tYEX19vUNAR8bWY531vLA7AefWETszXbw";
const STORE: &str = "BMXRBHpvGUh6uhYjReyUF9Z8N9xVvhw4qGX5u2ekUS7g";
const METAPLEX_PROGRAM: &str = "4hasEWaJeuhvYkayMSkchJ4xLz5Kbt4LyzZxpXigSfHq";
const METADATA_PROGRAM: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

fn make_auction(
    token_mint: Pubkey,
    token_account:Pubkey,
) {

    let vault_program = Pubkey::from_str(PROGRAM_PUBKEY).unwrap();
    let auction_program = Pubkey::from_str(AUCTION_PROGRAM).unwrap();
    let metadata_program = Pubkey::from_str(METADATA_PROGRAM).unwrap();
    let token_key = Pubkey::from_str(TOKEN_PROGRAM_PUBKEY).unwrap();
    let price_mint = Pubkey::from_str(WSOL).unwrap();
    let client = RpcClient::new("https://api.devnet.solana.com".to_string());    
    let payer = Keypair::from_base58_string(SECRET_KEY);
    let payer_key = payer.pubkey();
    let store_key =  Pubkey::from_str(STORE).unwrap();

    println!("Processing ==> update_external_account");
    let epa = Keypair::new();
    let mut signers_epa = vec![&payer];
    let mut instructions_epa = vec![];
    let create_epa_instruction = create_account(
        &payer.pubkey(),
        &epa.pubkey(),
        client
            .get_minimum_balance_for_rent_exemption(MAX_EXTERNAL_ACCOUNT_SIZE)
            .unwrap(),
        MAX_EXTERNAL_ACCOUNT_SIZE as u64,
        &vault_program,
    );
    signers_epa.push(&epa);
    instructions_epa.push(create_epa_instruction);
    instructions_epa.push(create_update_external_price_account_instruction(
        vault_program,
        epa.pubkey(),
        0,
        price_mint,
        true,
    ));

    let mut transaction_update_epa = Transaction::new_with_payer(&instructions_epa, Some(&payer.pubkey()));
    let recent_blockhash_epa = client.get_recent_blockhash().unwrap().0;

    transaction_update_epa.sign(&signers_epa, recent_blockhash_epa);
    let signature_epa = client.send_and_confirm_transaction(&transaction_update_epa).unwrap();
    println!("signature_epa:::{:?}",signature_epa);
    
    println!("Processing ==> init_vault");
    let mut signers = vec![&payer];
    let mut instructions = vec![];
    let fraction_mint = Keypair::new();
    signers.push(&fraction_mint);
    instructions.push(
        create_account(
            &payer.pubkey(),
            &fraction_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .unwrap(),
            Mint::LEN as u64,
            &token_key,
        ),
    );

    let redeem_treasury = Keypair::new();
    signers.push(&redeem_treasury);
    instructions.push(
        create_account(
            &payer.pubkey(),
            &redeem_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
    );

    let fraction_treasury = Keypair::new();
    signers.push(&fraction_treasury);
    instructions.push(
        create_account(
            &payer.pubkey(),
            &fraction_treasury.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
    );

    let vault = Keypair::new();
    signers.push(&vault);
    instructions.push(
        create_account(
            &payer.pubkey(),
            &vault.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(MAX_VAULT_SIZE)
                .unwrap(),
            MAX_VAULT_SIZE as u64,
            &vault_program,
        ),
    );
    let vault_pubkey = vault.pubkey();
    let seeds = &[PREFIX.as_bytes(), vault_program.as_ref(), vault_pubkey.as_ref()];
    let (authority, _) = Pubkey::find_program_address(seeds, &vault_program);

    instructions.push(
        initialize_mint(
            &token_key,
            &fraction_mint.pubkey(),
            &authority,
            Some(&authority),
            0,
        )
        .unwrap(),
    );

    instructions.push(
        initialize_account(
            &token_key,
            &redeem_treasury.pubkey(),
            &price_mint,
            &authority,
        )
        .unwrap(),
    );

    instructions.push(
        initialize_account(
            &token_key,
            &fraction_treasury.pubkey(),
            &fraction_mint.pubkey(),
            &authority,
        )
        .unwrap(),
    );

    instructions.push(
        create_init_vault_instruction(
            vault_program,
            fraction_mint.pubkey(),
            redeem_treasury.pubkey(),
            fraction_treasury.pubkey(),
            vault.pubkey(),
            payer.pubkey(),
            epa.pubkey(),
            true,
        ),
    );

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;

    transaction.sign(&signers, recent_blockhash);
    let signature = client.send_and_confirm_transaction(&transaction).unwrap();
    println!("vault:::{:?}",vault_pubkey);
    println!("signature:::{:?}",signature);


    println!("Processing ==> add_token_to_vault");
    let store = Keypair::new();
    let transfer_authority = Keypair::new();
    let seeds = &[
        PREFIX.as_bytes(),
        vault_pubkey.as_ref(),
        token_mint.as_ref(),
    ];
    let (safety_deposit_box, _) = Pubkey::find_program_address(seeds, &vault_program);


    let instructions_add = [
        create_account(
            &payer.pubkey(),
            &store.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Account::LEN)
                .unwrap(),
            Account::LEN as u64,
            &token_key,
        ),
        initialize_account(
            &token_key,
            &store.pubkey(),
            &token_mint,
            &authority,
        )
        .unwrap(),
        approve(
            &token_key,
            &token_account,
            &transfer_authority.pubkey(),
            &payer.pubkey(),
            &[&payer.pubkey()],
            1,
        )
        .unwrap(),
        create_add_token_to_inactive_vault_instruction(
            vault_program,
            safety_deposit_box,
            token_account,
            store.pubkey(),
            vault_pubkey,
            payer.pubkey(),
            payer.pubkey(),
            transfer_authority.pubkey(),
            1,
        ),
    ];

    let signers_add = vec![
        &payer,
        &store,
        &transfer_authority,
    ];


    let mut transaction_add = Transaction::new_with_payer(&instructions_add, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction_add.sign(&signers_add, recent_blockhash);
    client.send_and_confirm_transaction(&transaction_add).unwrap();
    let safety_deposit_account = client.get_account(&safety_deposit_box).unwrap();
    println!("safety_deposit_account:::{:?}",&safety_deposit_account);


    println!("Processing ==> activate_vault_combine_vault");
    let outstanding_shares_account = Keypair::new();
    let payment_account = Keypair::new();
    let transfer_authority_add_combine = Keypair::new();
    
    let instructions_activate_combine = vec![
        create_activate_vault_instruction(
        vault_program,
        vault_pubkey,
        fraction_mint.pubkey(),
        fraction_treasury.pubkey(),
        authority,
        payer.pubkey(),
        0,
    ),
    create_account(
        &payer.pubkey(),
        &payment_account.pubkey(),
        client
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .unwrap(),
        Account::LEN as u64,
        &token_key,
    ),
    initialize_account(
        &token_key,
        &payment_account.pubkey(),
        &price_mint,
        &payer.pubkey(),
    )
    .unwrap(),
    create_account(
        &payer.pubkey(),
        &outstanding_shares_account.pubkey(),
        client
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .unwrap(),
        Account::LEN as u64,
        &token_key,
    ),
    initialize_account(
        &token_key,
        &outstanding_shares_account.pubkey(),
        &fraction_mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap(),
    approve(
        &token_key,
        &outstanding_shares_account.pubkey(),
        &transfer_authority_add_combine.pubkey(),
        &payer.pubkey(),
        &[&payer.pubkey()],
        0,
    )
    .unwrap(),
    approve(
        &token_key,
        &payment_account.pubkey(),
        &transfer_authority_add_combine.pubkey(),
        &payer.pubkey(),
        &[&payer.pubkey()],
        0,
    )
    .unwrap(),
    create_combine_vault_instruction(
        vault_program,
        vault_pubkey,
        outstanding_shares_account.pubkey(),
        payment_account.pubkey(),
        fraction_mint.pubkey(),
        fraction_treasury.pubkey(),
        redeem_treasury.pubkey(),
        payer.pubkey(),
        payer.pubkey(),
        transfer_authority_add_combine.pubkey(),
        authority,
        epa.pubkey(),
    )
    ];

    let mut transaction_activate_combine = Transaction::new_with_payer(&instructions_activate_combine, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    let signers_add_combine = vec![&payer,&payment_account,&outstanding_shares_account,&transfer_authority_add_combine ];
    transaction_activate_combine.sign(&signers_add_combine, recent_blockhash);
    let signature_activate = client.send_and_confirm_transaction(&transaction_activate_combine).unwrap();
    // let updated_vault_data = client.get_account(&vault_pubkey).unwrap();
    // let updated_vault: Vault = try_from_slice_unchecked(&updated_vault_data.data).unwrap();
    // if updated_vault.state == VaultState::Combined {
    //     println!("Combined vault.");
    // } else {
    //     println!("Failed to combined vault.");
    // }
    println!("signature_activate::: {:?}",signature_activate);


    println!("Processing ==> create_auction");
    let create_instruction = [create_auction_instruction_v2(
        auction_program,
        payer.pubkey(),
        CreateAuctionArgsV2 {
            authority: payer.pubkey(),
            end_auction_at: None,
            end_auction_gap: None,
            resource: vault_pubkey,
            token_mint: price_mint,
            winners: WinnerLimit::Capped(1),
            price_floor: PriceFloor::None([0; 32]),
            gap_tick_size_percentage: None,
            tick_size: None,
            name: None,
            instant_sale_price: Some(2000000000),
        },
    )];

    let signers_create = vec![&payer];
    let mut transaction_create = Transaction::new_with_payer(&create_instruction, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction_create.sign(&signers_create, recent_blockhash);
    let signature_create = client.send_and_confirm_transaction(&transaction_create).unwrap();
    println!("signature_create:::{:?}",&signature_create);

    
    println!("Processing ==> init_auction_managerV2");

    let metaplex_program = Pubkey::from_str(METAPLEX_PROGRAM).unwrap();
    let accept_payment = Keypair::new();

    let seeds = &["auction".as_bytes(), auction_program.as_ref(), vault_pubkey.as_ref()];
    let (auction_key, _) = Pubkey::find_program_address(seeds, &auction_program);

    let seeds = &["metaplex".as_bytes(), auction_key.as_ref()];
    let (auction_manager_key, _) = Pubkey::find_program_address(seeds, &metaplex_program);

    let seeds = &["metaplex".as_bytes(), metaplex_program.as_ref(), auction_manager_key.as_ref(), "totals".as_bytes()];
    let (token_tracker, _) = Pubkey::find_program_address(seeds, &metaplex_program);

    #[allow(clippy::too_many_arguments)]
    let instructions_init_auction_manager = vec![
    create_account(
        &payer.pubkey(),
        &accept_payment.pubkey(),
        client
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .unwrap(),
        Account::LEN as u64,
        &token_key,
    ),
    initialize_account(
        &token_key,
        &accept_payment.pubkey(),
        &price_mint,
        &auction_manager_key,
    )
    .unwrap(),
    #[allow(clippy::too_many_arguments)]
    create_init_auction_manager_v2_instruction(
        metaplex_program,
        token_tracker,   
        auction_manager_key,
        vault_pubkey,
        auction_key,
        payer.pubkey(),
        payer.pubkey(),
        accept_payment.pubkey(),
        store_key,
        TupleNumericType::U8,
        TupleNumericType::U8,
        1,
    )
    ];

    let signers = vec![&payer,&accept_payment];
    let mut transaction_init = Transaction::new_with_payer(&instructions_init_auction_manager, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction_init.sign(&signers, recent_blockhash);
    let signature = client.send_and_confirm_transaction(&transaction_init).unwrap();
    println!("signature_init_auction_manager:::{:?}",&signature);
    
    println!("Processing ==> set_authority");
    let instructions_set_authority = vec![
        create_set_authority_instruction(
            vault_program,
            vault_pubkey,
            payer.pubkey(),
            auction_manager_key,
        ),
        set_authority_instruction(
            auction_program,
            vault_pubkey,
            payer.pubkey(),
            auction_manager_key,
        ),
    ];

    let signers = vec![&payer];
    let mut transaction = Transaction::new_with_payer(&instructions_set_authority, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&signers, recent_blockhash);
    let signature_set_authority = client.send_and_confirm_transaction(&transaction).unwrap();
    println!("signature_set_authority:::{:?}",&signature_set_authority);


    println!("Processing ==> validate");
    let seeds = &["metadata".as_bytes(), metadata_program.as_ref(), token_mint.as_ref()];
    let (metadata, _) = Pubkey::find_program_address(seeds, &metadata_program);

    let seeds = &["metadata".as_bytes(), metadata_program.as_ref(), token_mint.as_ref(), "edition".as_bytes()];
    let (edition, _) = Pubkey::find_program_address(seeds, &metadata_program);

    let seeds = &["metaplex".as_bytes(), metaplex_program.as_ref(), store_key.as_ref(),payer_key.as_ref() ];
    let (whitelisted_creator, _) = Pubkey::find_program_address(seeds, &metaplex_program);

    let seeds = &["metaplex".as_bytes(), auction_key.as_ref(), metadata.as_ref()];
    let (original_authority_lookup, _) = Pubkey::find_program_address(seeds, &metaplex_program);

    // let seeds = &["metaplex".as_bytes(), metaplex_program.as_ref(), auction_manager_key.as_ref(),safety_deposit_box.as_ref()];
    // let (validation, _) = Pubkey::find_program_address(seeds, &metaplex_program);

    let amount_ranges = AmountRange(1, 1);

    let safety_deposit_config = SafetyDepositConfig {
        key: Key::SafetyDepositConfigV1,
        auction_manager: auction_manager_key,
        order: 0,
        winning_config_type: WinningConfigType::FullRightsTransfer,
        amount_type: TupleNumericType::U16,
        length_type: TupleNumericType::U16,
        amount_ranges: vec![amount_ranges],
        participation_config: None,
        participation_state: None,
    };

#[allow(clippy::too_many_arguments)]
    let instructions_validate = [
        create_validate_safety_deposit_box_v2_instruction(
            metaplex_program, 
            auction_manager_key,
            token_tracker,
            metadata,
            original_authority_lookup,
            whitelisted_creator,
            store_key,
            safety_deposit_box,
            store.pubkey(),
            token_mint,
            edition,
            vault_pubkey,
            payer_key,
            payer_key,
            payer_key,
            safety_deposit_config,
        ),
    ];
    
    let signers = vec![&payer];
    let mut transaction = Transaction::new_with_payer(&instructions_validate, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&signers, recent_blockhash);
    let signature_validate = client.send_and_confirm_transaction(&transaction).unwrap();
    println!("signature_validate:::{:?}",&signature_validate);


    println!("Processing ==> start_auction");
    let instructions_start_auction = [
        create_start_auction_instruction(
            metaplex_program,
            auction_manager_key,
            auction_key,
            auction_program,
            payer_key,
            store_key,
        ),
    ];

    let signers = vec![&payer];
    let mut transaction = Transaction::new_with_payer(&instructions_start_auction, Some(&payer.pubkey()));
    let recent_blockhash = client.get_recent_blockhash().unwrap().0;
    transaction.sign(&signers, recent_blockhash);
    let signature_start_auction = client.send_and_confirm_transaction(&transaction).unwrap();
    println!("signature_start_auction:::{:?}",&signature_start_auction);
    println!("Auction started yet::: well done--------------------------------");   
}

fn main() {

    println!("HERE WE GO, FIRE IN THE HOLE!!!");
    let token = "6geVPkPbzaBdPMMJp8fcdRVQCqnmErBa1CDUuPGTNFn4";
    let acc = "DdZxA1N1fBTy3e6Wm2pf3oQF83J1yXebgXZGKhY3SwLS";
    let _mint = Pubkey::from_str(token).unwrap();
    let account = Pubkey::from_str(acc).unwrap();
    make_auction(_mint,account);

}
