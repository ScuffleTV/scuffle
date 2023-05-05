mkdir -p rsa ec

openssl genrsa -out rsa/ca.key 2048
openssl genrsa -out rsa/server.key 2048
openssl genrsa -out rsa/client.key 2048

openssl req -x509 -sha256 -days 365 -nodes -key rsa/ca.key -config ca.ini -out rsa/ca.crt
openssl req -x509 -sha256 -days 365 -CA rsa/ca.crt -CAkey rsa/ca.key -nodes -key rsa/server.key -config server.ini -out rsa/server.crt
openssl req -x509 -sha256 -days 365 -CA rsa/ca.crt -CAkey rsa/ca.key -nodes -key rsa/client.key -config client.ini -out rsa/client.crt

openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out ec/ca.key
openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out ec/server.key
openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out ec/client.key

openssl pkcs8 -topk8 -nocrypt -in ec/ca.key -out ec/ca.key.pem
openssl pkcs8 -topk8 -nocrypt -in ec/server.key -out ec/server.key.pem
openssl pkcs8 -topk8 -nocrypt -in ec/client.key -out ec/client.key.pem

mv ec/ca.key.pem ec/ca.key
mv ec/server.key.pem ec/server.key
mv ec/client.key.pem ec/client.key

openssl req -x509 -sha256 -days 365 -nodes -key ec/ca.key -config ca.ini -out ec/ca.crt
openssl req -x509 -sha256 -days 365 -CA ec/ca.crt -CAkey ec/ca.key -nodes -key ec/server.key -config server.ini -out ec/server.crt
openssl req -x509 -sha256 -days 365 -CA ec/ca.crt -CAkey ec/ca.key -nodes -key ec/client.key -config client.ini -out ec/client.crt
