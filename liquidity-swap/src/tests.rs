#[cfg(test)]
mod test {
    use crate::{
        calculate_equivalent_and_minted_tokens, calculate_reclaim_output, calculate_swap_to_amount,
        u128_sqrt,
    };
    use rand::Rng;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    pub fn test_u128_sqrt() {
        assert_eq!(u128_sqrt(25), 5);
        assert_eq!(u128_sqrt(20), 4);
        assert_eq!(u128_sqrt(0), 0);
        assert_eq!(u128_sqrt(1), 1);
    }

    #[test]
    pub fn test_calculate_swap_to_amount() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
        // Using a range of [1, 1000] ensures that the swap fee, will at most amount to 3 tokens
        let end_input_range = 1000;

        // Equal start values for clarity. Large pools for reduced sensitivity.
        let a_pool: u128 = 123456789;
        let b_pool: u128 = 123456789;

        for _ in 0..=1000000 {
            let input_a: u128 = rng.gen_range(1..=end_input_range);

            // Swap back and forth
            // First swap will give us an input/output difference of at most 4, if input is 1000, due to a fee of 3 and flooring.
            let output_b: u128 = calculate_swap_to_amount(a_pool, b_pool, input_a, 3);
            // Second swap will give us an input/output difference of at most 3, if first input was 1000, due to a fee of strictly less than 3 and flooring.
            let output_a: u128 =
                calculate_swap_to_amount(b_pool - output_b, a_pool + input_a, output_b, 3);

            assert!(
                output_a + 3 >= output_b,
                "Output_a was: {output_a}, output_b was: {output_b}"
            );

            // Original input comparison
            assert!(
                output_a + 7 >= input_a,
                "Output_a was: {output_a}, input_a was: {input_a}"
            );
        }
    }

    #[test]
    pub fn calculate_swap_to_amount_float_floored_should_be_equal() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
        let end_input_range = 10000000;

        for _ in 0..=100000 {
            let input_a: u64 = rng.gen_range(1..=end_input_range);
            let a_pool: u64 = rng.gen_range(1..=end_input_range);
            let b_pool: u64 = rng.gen_range(1..=end_input_range);

            let output_b: u128 =
                calculate_swap_to_amount(a_pool as u128, b_pool as u128, input_a as u128, 3);

            // Calculate_swap_to_amount using floats and flooring in the end
            let input_a_flt: f64 = input_a as f64;
            let trade_fee_flt: f64 = input_a_flt * 0.003;
            let input_a_after_fee_flt: f64 = input_a_flt - trade_fee_flt;
            let output_b_flt: f64 =
                b_pool as f64 * (input_a_after_fee_flt) / (a_pool as f64 + input_a_after_fee_flt);
            let output_b_flt_floor: u128 = output_b_flt.floor() as u128;

            assert_eq!(
                output_b_flt_floor, output_b,
                "Output_b_flt_floor was: {output_b_flt_floor}, output_b was: {output_b}"
            );
        }
    }

    #[test]
    pub fn test_calculate_equivalent_and_minted_tokens() {
        // Equal token values, providing 10% of token A
        let pool_a: u128 = 100;
        let pool_b: u128 = 100;
        let total_minted_liquidity: u128 = 100;
        let provided_amount: u128 = 10;

        let (output_b, output_liquidity_tokens) = calculate_equivalent_and_minted_tokens(
            provided_amount,
            pool_a,
            pool_b,
            total_minted_liquidity,
        );

        assert_eq!(output_b, 11); // Explicit case of depositing an additional token, despite not being necessary if using float arithmetic
        assert_eq!(output_liquidity_tokens, 10);

        let pool_b: u128 = 99;
        let (new_output_b, _) = calculate_equivalent_and_minted_tokens(
            provided_amount,
            pool_a,
            pool_b,
            total_minted_liquidity,
        );

        assert_eq!(new_output_b, 10); // Lowering the ratio of the pool tokens slightly gives expected output

        // Equal token values, providing (approximately) 10% of token A
        let pool_a: u128 = 99999;
        let pool_b: u128 = 99999;
        let total_minted_lliquidity: u128 = 100;
        let provided_amount: u128 = 9999;

        let (output_b, output_liquidity_tokens) = calculate_equivalent_and_minted_tokens(
            provided_amount,
            pool_a,
            pool_b,
            total_minted_lliquidity,
        );

        assert_eq!(output_b, 10000);
        assert_eq!(output_liquidity_tokens, 9); // Explicit case of minting 1 less token, despite being very close to expected value of 10
    }

    #[test]
    pub fn test_calculate_updated_liquidity_reclaim() {
        // Equal token values, reclaiming 10% of total shares
        let pool_a: u128 = 100;
        let pool_b: u128 = 100;
        let total_minted_liquidity: u128 = 100;
        let liquidity_tokens: u128 = 10;

        let (a_output, b_output) =
            calculate_reclaim_output(liquidity_tokens, pool_a, pool_b, total_minted_liquidity);

        assert_eq!(a_output, 10);
        assert_eq!(b_output, 10);

        // Token A worth 5 token B, reclaiming 10% of total shares
        let pool_a: u128 = 30;
        let pool_b: u128 = 150;
        let total_minted_liquidity: u128 = 100;
        let liquidity_tokens: u128 = 10;

        let (output_a, output_b) =
            calculate_reclaim_output(liquidity_tokens, pool_a, pool_b, total_minted_liquidity);

        assert_eq!(output_a, 3);
        assert_eq!(output_b, 15);

        // Token A worth 2 token B, reclaiming 25% of total shares
        let pool_a: u128 = 100;
        let pool_b: u128 = 200;
        let total_minted_liquidity: u128 = 100;
        let liquidity_tokens: u128 = 25;

        let (output_a, output_b) =
            calculate_reclaim_output(liquidity_tokens, pool_a, pool_b, total_minted_liquidity);

        assert_eq!(output_a, 25);
        assert_eq!(output_b, 50);
    }

    #[test]
    pub fn calculate_equivalent_and_minted_tokens_stress_test() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
        let end_range = 10000;
        for _ in 0..101 {
            let mut pool_a: u128 = rng.gen_range(1..=end_range);
            let mut pool_b: u128 = rng.gen_range(1..=end_range);
            let mut total_minted_liquidity: u128 = u128_sqrt(pool_a * pool_b);
            let mut constant_product = pool_a * pool_b;

            for _ in 0..10001 {
                let provided_a_tokens = rng.gen_range(1..=end_range);
                let provided_b_tokens_float_floor = pool_b * provided_a_tokens / pool_a;
                let minted_liquidity_float_floor =
                    total_minted_liquidity * provided_a_tokens / pool_a;

                let (provided_b_tokens, minted_liquidity) = calculate_equivalent_and_minted_tokens(
                    provided_a_tokens,
                    pool_a,
                    pool_b,
                    total_minted_liquidity,
                );

                // Check invariants
                assert_eq!(provided_b_tokens, provided_b_tokens_float_floor + 1);
                assert_eq!(minted_liquidity, minted_liquidity_float_floor);

                assert!(pool_a < pool_a + provided_a_tokens);
                assert!(pool_b < pool_b + provided_b_tokens);
                assert!(total_minted_liquidity <= total_minted_liquidity + minted_liquidity); // Can happen that nothing is minted
                assert!(
                    constant_product < (pool_a + provided_a_tokens) * (pool_b + provided_b_tokens)
                );

                // Update state
                pool_a += provided_a_tokens;
                pool_b += provided_b_tokens;
                total_minted_liquidity += minted_liquidity;
                constant_product = pool_a * pool_b;
            }
        }
    }

    #[test]
    pub fn calculate_calculate_reclaim_output_stress_test() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
        for _ in 0..1001 {
            let mut pool_a: u128 = rng.gen_range(1..10001);
            let mut pool_b: u128 = rng.gen_range(1..10001);
            let mut total_minted_liquidity: u128 = u128_sqrt(pool_a * pool_b);
            let mut constant_product = pool_a * pool_b;

            for _ in 0..101 {
                let total_minted_liquidity_copy = total_minted_liquidity; // immutable range
                let provided_minted_liquidity = rng.gen_range(1..total_minted_liquidity_copy);

                let output_a_float_floor =
                    pool_a * provided_minted_liquidity / total_minted_liquidity;
                let output_b_float_floor =
                    pool_b * provided_minted_liquidity / total_minted_liquidity;

                let (output_a, output_b) = calculate_reclaim_output(
                    provided_minted_liquidity,
                    pool_a,
                    pool_b,
                    total_minted_liquidity,
                );

                // Check invariants
                assert_eq!(output_a, output_a_float_floor);
                assert_eq!(output_b, output_b_float_floor);

                assert!(pool_a - output_a <= pool_a);
                assert!(pool_b - output_b <= pool_b);
                assert!(
                    total_minted_liquidity - provided_minted_liquidity < total_minted_liquidity
                );
                assert!(output_a * output_b <= constant_product);

                // Update state
                pool_a -= output_a;
                pool_b -= output_b;
                total_minted_liquidity -= provided_minted_liquidity;
                constant_product = pool_a * pool_b;

                // Stop early if we cannot reclaim more than 1 token
                if total_minted_liquidity <= 1 {
                    break;
                }
            }
        }
    }

    #[test]
    pub fn zero_cases() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(10);
        // Some state
        let e: u128 = rng.gen_range(1..10001);
        let t: u128 = rng.gen_range(1..10001);
        let l: u128 = u128_sqrt(e * t);
        let k = e * t;

        let delta_e: u128 = 0;
        let delta_t = calculate_swap_to_amount(e, t, delta_e, 3);

        // State remains unchanged as a result of delta_e and delta_t being 0
        assert_eq!(delta_t, 0);

        let (opposite_equivalent, minted_liquidity_tokens) =
            calculate_equivalent_and_minted_tokens(0, e, t, l);
        assert_eq!(opposite_equivalent, 0);
        assert_eq!(minted_liquidity_tokens, 0);

        let (a_output, b_output) = calculate_reclaim_output(0, e, t, l);
        assert_eq!(a_output, 0);
        assert_eq!(b_output, 0);
    }
}
