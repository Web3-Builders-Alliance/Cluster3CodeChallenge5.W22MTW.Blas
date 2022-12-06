#![cfg(test)]

use cosmwasm_std::{to_binary, Addr, Empty, Uint128, WasmMsg};
use counter::msg::GetCountResponse;
use cw20::{BalanceResponse, MinterResponse};
use cw20_base::msg::QueryMsg;
use cw3::Vote;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::{Duration, Threshold};

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, Voter};

use counter::{self, helpers::CounterContract};

fn mock_app() -> App {
    App::default()
}

pub fn contract_cw3_fixed_multisig() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_counter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        counter::contract::execute,
        counter::contract::instantiate,
        counter::contract::query,
    );
    Box::new(contract)
}

fn store_code() -> (App, u64, u64) {
    let mut app = mock_app();
    let cw3_multisig = app.store_code(contract_cw3_fixed_multisig());
    let counter_id = app.store_code(contract_counter());
    (app, cw3_multisig, counter_id)
}

#[test]
// cw3 multisig account can control cw20 admin actions
fn cw3_controls_cw20() {
    let mut router = mock_app();

    // setup cw3 multisig with 3 accounts
    let cw3_id = router.store_code(contract_cw3_fixed_multisig());

    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");
    let addr3 = Addr::unchecked("addr3");
    let cw3_instantiate_msg = InstantiateMsg {
        voters: vec![
            Voter {
                addr: addr1.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr2.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr3.to_string(),
                weight: 1,
            },
        ],
        threshold: Threshold::AbsoluteCount { weight: 2 },
        max_voting_period: Duration::Height(3),
    };

    let multisig_addr = router
        .instantiate_contract(
            cw3_id,
            addr1.clone(),
            &cw3_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // setup cw20 as cw3 multisig admin
    let cw20_id = router.store_code(contract_cw20());

    let cw20_instantiate_msg = cw20_base::msg::InstantiateMsg {
        name: "Consortium Token".parse().unwrap(),
        symbol: "CST".parse().unwrap(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: multisig_addr.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let cw20_addr = router
        .instantiate_contract(
            cw20_id,
            multisig_addr.clone(),
            &cw20_instantiate_msg,
            &[],
            "Consortium",
            None,
        )
        .unwrap();

    // mint some cw20 tokens according to proposal result
    let mint_recipient = Addr::unchecked("recipient");
    let mint_amount = Uint128::new(1000);
    let cw20_mint_msg = cw20_base::msg::ExecuteMsg::Mint {
        recipient: mint_recipient.to_string(),
        amount: mint_amount,
    };

    let execute_mint_msg = WasmMsg::Execute {
        contract_addr: cw20_addr.to_string(),
        msg: to_binary(&cw20_mint_msg).unwrap(),
        funds: vec![],
    };
    let propose_msg = ExecuteMsg::Propose {
        title: "Mint tokens".to_string(),
        description: "Need to mint tokens".to_string(),
        msgs: vec![execute_mint_msg.into()],
        latest: None,
    };
    // propose mint
    router
        .execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
        .unwrap();

    // second votes
    let vote2_msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: Vote::Yes,
    };
    router
        .execute_contract(addr2, multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();

    // only 1 vote and msg mint fails
    let execute_proposal_msg = ExecuteMsg::Execute { proposal_id: 1 };
    // execute mint
    router
        .execute_contract(addr1, multisig_addr, &execute_proposal_msg, &[])
        .unwrap();

    // check the mint is successful
    let cw20_balance_query = QueryMsg::Balance {
        address: mint_recipient.to_string(),
    };
    let balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(&cw20_addr, &cw20_balance_query)
        .unwrap();

    // compare minted amount
    assert_eq!(balance.balance, mint_amount);
}

///CW3 Controls Counter
#[test]
fn cw3_controls_counter() {
    //CODING CHALLENEGE
    let mut router = mock_app();

    // setup cw3 multisig with 3 accounts
    let cw3_id = router.store_code(contract_cw3_fixed_multisig());

    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");
    let addr3 = Addr::unchecked("addr3");
    let addr4 = Addr::unchecked("addr4");
    let addr5 = Addr::unchecked("addr5");
    let cw3_instantiate_msg = InstantiateMsg {
        voters: vec![
            Voter {
                addr: addr1.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr2.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr3.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr4.to_string(),
                weight: 1,
            },
            Voter {
                addr: addr5.to_string(),
                weight: 1,
            },
        ],
        threshold: Threshold::AbsoluteCount { weight: 2 },
        max_voting_period: Duration::Height(3),
    };


    let multisig_addr = router
        .instantiate_contract(
            cw3_id,
            addr1.clone(),
            &cw3_instantiate_msg,
            &[],
            "3 out of 5 MultiSig Contract",
            None,
        )
        .unwrap();

    // setup cw20 as cw3 multisig admin
    let counter_id = router.store_code(contract_counter());

    let counter_instantiate_msg = counter::msg::InstantiateMsg {
        count: 0,
        owner: multisig_addr.clone().to_string(),
    };

    let counter_addr = router
        .instantiate_contract(
            counter_id,
            multisig_addr.clone(),
            &counter_instantiate_msg,
            &[],
            "Counter for MultiSig Contract",
            None,
        )
        .unwrap();

    
    // FIRST PROPOSAL: Increase Counter Message Proposal
    let counter_increase_msg = counter::msg::ExecuteMsg::Increment {  };

    let execute_counter_increase_msg = WasmMsg::Execute {
        contract_addr: counter_addr.to_string(),
        msg: to_binary(&counter_increase_msg).unwrap(),
        funds: vec![],
    };

    let propose_msg = ExecuteMsg::Propose {
        title: "Increase Counter".to_string(),
        description: "Increase the counter by 1".to_string(),
        msgs: vec![execute_counter_increase_msg.into()],
        latest: None,
    };
    // propose mint, this will already vote like 1.
    router
        .execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
        .unwrap();

    // second votes. Now we have got 2.
    let vote2_msg = ExecuteMsg::Vote {
        proposal_id: 1,
        vote: Vote::Yes,
    };
    router
        .execute_contract(addr2.clone(), multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();
    
    // Query the counter to make sure it is still zero
    let counter_count_query = counter::msg::QueryMsg::GetCount {  };
    let count: GetCountResponse = router
        .wrap()
        .query_wasm_smart(&counter_addr, &counter_count_query)
        .unwrap();
    assert_eq!(count.count, 0);

    // Third vote. Now we have got 3.
    router
        .execute_contract(addr3.clone(), multisig_addr.clone(), &vote2_msg, &[])
        .unwrap();


    // The multisig contract is ready to Execute the Proposal with id:1.
    let execute_proposal_msg = ExecuteMsg::Execute { proposal_id: 1 };
    // execute mint
    router
        .execute_contract(addr1.clone(), multisig_addr.clone(), &execute_proposal_msg, &[])
        .unwrap();

    // Query the counter to make sure it is now 1.
    let counter_count_query = counter::msg::QueryMsg::GetCount {  };
    let count: GetCountResponse = router
        .wrap()
        .query_wasm_smart(&counter_addr, &counter_count_query)
        .unwrap();
    assert_eq!(count.count, 1);   


       // SECOND PROPOSAL: Reset Counter
       let counter_reset_msg = counter::msg::ExecuteMsg::Reset { count: 0 };

       let execute_counter_reset_msg = WasmMsg::Execute {
           contract_addr: counter_addr.to_string(),
           msg: to_binary(&counter_reset_msg).unwrap(),
           funds: vec![],
       };
   
       let propose_msg = ExecuteMsg::Propose {
           title: "Reset Counter".to_string(),
           description: "Reset counter to 0".to_string(),
           msgs: vec![execute_counter_reset_msg.into()],
           latest: None,
       };
       // propose mint, this will already vote like 1. This new proposal will be created with an id of 2.
       router
           .execute_contract(addr1.clone(), multisig_addr.clone(), &propose_msg, &[])
           .unwrap();
   
       // second votes. Now we have got 2.
       let vote2_msg = ExecuteMsg::Vote {
           proposal_id: 2,
           vote: Vote::Yes,
       };
       router
           .execute_contract(addr2, multisig_addr.clone(), &vote2_msg, &[])
           .unwrap();
       
       // Query the counter to make sure it is still one as the proposal has not passed.
       let counter_count_query = counter::msg::QueryMsg::GetCount {  };
       let count: GetCountResponse = router
           .wrap()
           .query_wasm_smart(&counter_addr, &counter_count_query)
           .unwrap();
       assert_eq!(count.count, 1);
   
       // Third vote. Now we have got 3.
       router
           .execute_contract(addr3, multisig_addr.clone(), &vote2_msg, &[])
           .unwrap();
   
   
       // The multisig contract is ready to Execute the Proposal with id:2.
       let execute_proposal_msg = ExecuteMsg::Execute { proposal_id: 2 };
       // execute mint
       router
           .execute_contract(addr1, multisig_addr, &execute_proposal_msg, &[])
           .unwrap();
   
       // Query the counter to make sure it is now 0.
       let counter_count_query = counter::msg::QueryMsg::GetCount {  };
       let count: GetCountResponse = router
           .wrap()
           .query_wasm_smart(&counter_addr, &counter_count_query)
           .unwrap();
       assert_eq!(count.count, 0);   

}
