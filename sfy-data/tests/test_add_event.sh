HOST=http://157.249.74.12:3000
HOST=http://localhost:3000

curl -f -v -X POST "${HOST}/buoy" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  -d "{ \"event\": \"0000-test\", \"time\": \"$(date -u +%s)\", \"device\": \"dev:1234\" }"
