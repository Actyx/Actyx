#!/bin/bash -e

vault_version=1.0.3
# From https://releases.hashicorp.com/vault/1.0.3/vault_1.0.3_SHA256SUMS
if [ ! -x $HOME/bin/vault ]; then
  declare -A sha256sum
  sha256sum=(
    [amd64]="a475946872b1a4a2bd8ea79ea1dd00fe65aa502f45d734a07afc022bf2ba8bcf  vault.zip"
    [arm]="995e761c71a627e678e6ca95c5134fa18689b2f4fb532234919cb9a991b72d07  vault.zip"
  )

  case $(uname -m) in
    arm*)
      arch=arm
      ;;
    *)
      arch=amd64
      ;;
  esac

  echo "ARCH is ${arch}"

  mkdir -p $HOME/bin
  cd $HOME/bin
  curl -o vault.zip https://releases.hashicorp.com/vault/${vault_version}/vault_${vault_version}_linux_${arch}.zip
  sha256sum -b vault.zip
  echo "Expected sha256sum:"
  echo "${sha256sum[${arch}]}"
  if ! echo "${sha256sum[${arch}]}" | sha256sum --check; then
    echo "Wrong SHA256SUM for vault.zip. Aborting."
    exit -1
  fi

  # -o: overwrite w/o prompting
  unzip -o vault.zip
fi

export VAULT_ADDR=https://vault.actyx.net
out=$($HOME/bin/vault login -method aws role=ops-travis-ci 2>&1)

if [ $? -ne 0 ]; then
  echo "Failed to log in to Vault: $out"
  exit -1
fi

echo "Vault version $vault_version for architecture $arch installed"
