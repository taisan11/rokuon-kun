//音声エフェクトを作る
/// 音声データにダイナミックレンジコンプレッサーを適用する
///
/// # 引数
/// - `samples`: f32音声データ（-1.0〜1.0）
/// - `threshold_db`: スレッショルド（例: -20.0）
/// - `ratio`: レシオ（例: 4.0）
///
/// # 戻り値
/// - 圧縮後の f32 サンプル列
pub fn compress_audio(samples: &[f32], threshold_db: f32, ratio: f32) -> Vec<f32> {
    let mut result = Vec::with_capacity(samples.len());

    let threshold_amp = 10f32.powf(threshold_db / 20.0);

    for &sample in samples {
        let abs = sample.abs();

        let compressed = if abs > threshold_amp {
            let gain = threshold_amp + (abs - threshold_amp) / ratio;
            let gain_factor = gain / abs;
            sample * gain_factor
        } else {
            sample
        };

        result.push(compressed);
    }

    result
}