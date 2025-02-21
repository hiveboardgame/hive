docker build -t busybee .
docker run -d -p 8080:8080 --env-file /Users/leex/src/hive/busybee/.env --volume ./persistent/:/code/persistent busybee
