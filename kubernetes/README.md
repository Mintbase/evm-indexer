## Basic Usage (with kontemplate)

Install [kontemplate](https://code.tvl.fyi/tree/ops/kontemplate)

From within this directory

### Configure Secrets

1. Encode your secret(s)

```shell
echo -n 'Your secret value' | base64
```

2. Put into the [secrets files](./config/secrets.example.yaml) in place of `DO_NOT_ACTUALLY_PUT_SECRETS_HERE`


3. Apply secrets
```shell
kubectl apply -f config/secrets.yaml
```

### Delete Pod and Restart

```sh
kontemplate delete values.yaml
kontemplate apply values.yaml
```

or use the make file

```shell
make hard-restart
```

to check status and observe logs:

```sh
kubectl get pods
kubectl logs -f [POD_NAME]
```

or, for convenience:

```shell
kubectl logs -f $( kubectl get pods | grep arak-indexer | awk '{print $1}')
```