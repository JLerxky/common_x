use rand::Rng;
use rand_chacha::ChaCha8Rng;
use tracing::debug;

pub fn rand_one_in_vec(probability: &Vec<usize>, rng: &mut ChaCha8Rng) -> usize {
    let n = probability.len();
    if n == 1 {
        return 0;
    }
    let mut prob_sum = Vec::with_capacity(n);
    prob_sum.push(probability[0]);
    for i in 1..n {
        prob_sum.push(prob_sum[i - 1] + probability[i]);
    }
    debug!("{:?}", prob_sum);
    let r = rng.gen_range(1..=prob_sum[n - 1]);
    if r <= prob_sum[0] {
        return 0;
    }
    for i in 1..n {
        if r > prob_sum[i - 1] && r <= prob_sum[i] {
            return i;
        }
    }
    0
}

#[test]
fn test() {
    use rand::SeedableRng;
    use tracing::log::info;

    crate::log::init_log_filter("info");

    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let probability = vec![1, 1, 1, 1, 1];
    let mut count = vec![0; probability.len()];
    for _ in 0..10000 {
        let r = rand_one_in_vec(&probability, &mut rng);
        count[r] += 1;
    }
    info!("{:?}", count);
}
