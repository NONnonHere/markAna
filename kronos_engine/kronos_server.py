from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List
import torch
import pandas as pd
import random
import numpy as np

SEED = 42
random.seed(SEED)
np.random.seed(SEED)
torch.manual_seed(SEED)
if torch.cuda.is_available():
    torch.cuda.manual_seed(SEED)
    torch.cuda.manual_seed_all(SEED)
    torch.backends.cudnn.deterministic = True
    torch.backends.cudnn.benchmark = False

from model import Kronos, KronosTokenizer, KronosPredictor

app = FastAPI(title="Kronos AI Inference Service")

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"Loading Kronos on: {device}")

tokenizer = KronosTokenizer.from_pretrained("NeoQuasar/Kronos-Tokenizer-base")
model = Kronos.from_pretrained("NeoQuasar/Kronos-base").to(device)

predictor = KronosPredictor(model, tokenizer, device=device)

class CandlePayload(BaseModel):
    date: int
    open: float
    high: float
    low: float
    close: float
    volume: int

class PredictRequest(BaseModel):
    symbol: str
    candles: List[CandlePayload]

class PredictResponse(BaseModel):
    signal: str
    predicted_return_pct: float

@app.post("/predict", response_model=PredictResponse)
def predict(request: PredictRequest):
    if len(request.candles) < 30:
        raise HTTPException(status_code=400, detail="Insufficient candles for prediction")

    df = pd.DataFrame([c.model_dump() for c in request.candles])


    df['date'] = pd.to_datetime(df['date'], unit='s')

    current_close = df['close'].iloc[-1]

    # x_timestamp is the timeline of the data we are feeding in
    x_time = df['date']

    # y_timestamp is the timeline we want to forecast
    # We add 1 day to the last known date
    next_day = df['date'].iloc[-1] + pd.Timedelta(days=1)
    y_time = pd.Series([next_day])

    # We only need the OHLCV columns for the actual values
    val_df = df[['open', 'high', 'low', 'close', 'volume']]

    # RUN THE AI INFERENCE
    try:
        # Pass the values and explicit timestamps
        forecast_df = predictor.predict(
            val_df,
            pred_len=1,
            x_timestamp=x_time,
            y_timestamp=y_time,
        )

        predicted_close = forecast_df['close'].iloc[-1]
        predicted_return = ((predicted_close - current_close) / current_close) * 100.0

        if predicted_return > 0.5:
            signal = "Buy"
        elif predicted_return < -0.5:
            signal = "Sell"
        else:
            signal = "Hold"

        print(f"[{request.symbol}] AI Forecast: {predicted_close:.2f} | Expected Return: {predicted_return:.2f}% | Signal: {signal}")
        return PredictResponse(signal=signal, predicted_return_pct=predicted_return)

    except Exception as e:
        print(f"Kronos Prediction Error: {e}")
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8000)
