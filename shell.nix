with import ./nix/pkgs.nix {};
let merged-openssl = symlinkJoin { name = "merged-openssl"; paths = [ openssl.out openssl.dev ]; };
in stdenv.mkDerivation rec {
  name = "rust-env";
  env = buildEnv { name = name; paths = buildInputs; };

  buildInputs = [
    rustup
    clang
    llvm
    llvmPackages.libclang
    openssl
    cacert
    #podman-compose
    docker-compose
    postgresql_11
  ];
  shellHook = ''
  export LIBCLANG_PATH="${llvmPackages.libclang}/lib"
  export OPENSSL_DIR="${merged-openssl}"

  echo "Deploying local PostgreSQL"
  export PG_DATA=./pgsql-data
  export PG_PORT=5435
  if [ ! -d "$PG_DATA" ]; then
    initdb $PG_DATA --auth=trust
    echo "port = $PG_PORT" >> $PG_DATA/postgresql.conf
    echo "unix_socket_directories = '$PWD'" >> $PG_DATA/postgresql.conf
    pg_ctl start -D$PG_DATA -l postgresql.log
    psql --host=$PWD -p$PG_PORT -d postgres -c "create role \"appenddb\" with login password 'appenddb';"
    psql --host=$PWD -p$PG_PORT -d postgres -c "create database \"appenddb\" owner \"appenddb\";"
    for f in ./appenddb-db-postgres/migrations}*.sql
    do
      echo "Applying $f"
      psql --host=$PWD -p$PG_PORT -U appenddb -d appenddb -f $f
    done
  else 
    pg_ctl start -D$PG_DATA -l postgresql.log
  fi

  function finish {
    pg_ctl stop -D$PG_DATA
  }
  trap finish EXIT

  export DATABASE_URL=postgres://appenddb:appenddb@localhost/appenddb
  echo "Local database accessible by $DATABASE_URL"
  '';
}
