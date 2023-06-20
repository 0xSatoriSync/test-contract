#[cfg(test)]
mod tests {
    use crate::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    const USER1: &str = "USER1";
    const USER2: &str = "USER2";
    const USER3: &str = "USER3";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "usei";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER1),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

        let msg = InstantiateMsg { owner: Addr::unchecked(ADMIN), fixed_fee: Uint128::from(1000u128) };
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    mod execute {
        use super::*;
        use crate::msg::ExecuteMsg;

        #[test]
        fn send_duo() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::SendDuo {receiver1: Addr::unchecked(USER2), receiver2: Addr::unchecked(USER3)};
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(USER1), cosmos_msg).unwrap();
        }
    }
}
