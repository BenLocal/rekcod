ARG PYTHON_VERSION=3.11.9-bookworm
FROM python:${PYTHON_VERSION} AS base
WORKDIR /app

COPY . .
RUN python -m pip install -r requirements.txt -i https://pypi.tuna.tsinghua.edu.cn/simple/

EXPOSE 8000

CMD python3 -m uvicorn app:app --host=0.0.0.0 --port=8000