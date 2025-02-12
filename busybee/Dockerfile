FROM python:3.12

WORKDIR /code

COPY ./ /code/

RUN pip install --no-cache-dir --upgrade -r /code/requirements.txt

CMD ["bash", "/code/launch.sh"]
