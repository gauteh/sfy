# Processing scripts for SFY data

Set up and install the environment using e.g. [`mamba`](https://github.com/conda-forge/miniforge#mambaforge) (`conda` replacement).

Install using e.g.:

```
$ mamba create -f ../environment.yml  # or use `conda`.
$ conda activate sfy
$ pip install -e .
```

## Usage

Specify the server and read-token in a [`.env`](./.env-example) file:

```
SFY_SERVER="http://wavebug.met.no:3000/"
SFY_READ_TOKEN="secret"
SFY_DATA_CACHE="/tmp/sfy-cache"
```

Or set them as environment variables in e.g. `.bashrc`:

```
export SFY_SERVER='http://wavebug.met.no:3000'
export SFY_READ_TOKEN='secret'
export SFY_DATA_CACHE='/tmp/sfy-cache'
```

with the conda environment activate try it out with:

```
sfydata list
```

