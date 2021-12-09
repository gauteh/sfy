HOST=http://157.249.74.12:3000
HOST=http://localhost:3000

curl -f -v -X POST "${HOST}/buoy/cain" \
  -H "SFY_AUTH_TOKEN: ${SFY_AUTH_TOKEN}" \
  -d "{ \"test_data\": \"hello\", \"time\": \"$(date -u +%s)\" }"
