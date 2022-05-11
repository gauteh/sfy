# Processing scripts for SFY data

Install using e.g.:

```
$ pip install -e .
```

## Usage

Specify the server and read-token in a [`.env`](./.env-example) file:

```
SFY_SERVER="http://wavebug.met.no:3000/"
SFY_READ_TOKEN="secret"
SFY_DATA_CACHE="/tmp/sfy-cache"
```

Or set them as environment variables:

```
export SFY_SERVER='http://wavebug.met.no:3000'
export SFY_READ_TOKEN='secret'
export SFY_DATA_CACHE='/tmp/sfy-cache'
```

try out with:

```
sfydata list
```

