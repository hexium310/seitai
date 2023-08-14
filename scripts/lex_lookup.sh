set -e

commit=6c9f20dc915b17f5619340069889db0aa007fcdc
curl -sSL https://github.com/festvox/flite/archive/$commit.tar.gz | tar zx
cd flite-$commit
./configure
make
cd testsuite
make lex_lookup
cp lex_lookup ../..
cd ../..
rm -rf flite-$commit
