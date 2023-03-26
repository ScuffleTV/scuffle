openssl genrsa -out ca.rsa.key 2048
openssl genrsa -out server.rsa.key 2048
openssl genrsa -out client.rsa.key 2048

openssl req -x509 -sha256 -days 365 -nodes -key ca.rsa.key -config ca.ini -out ca.rsa.crt
openssl req -x509 -sha256 -days 365 -CA ca.rsa.crt -CAkey ca.rsa.key -nodes -key server.rsa.key -config server.ini -out server.rsa.crt
openssl req -x509 -sha256 -days 365 -CA ca.rsa.crt -CAkey ca.rsa.key -nodes -key client.rsa.key -config client.ini -out client.rsa.crt

openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out ca.ec.key
openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out server.ec.key
openssl ecparam -outform PEM -name prime256v1 -genkey -noout -out client.ec.key

openssl pkcs8 -topk8 -nocrypt -in ca.ec.key -out ca.ec.key.pem
openssl pkcs8 -topk8 -nocrypt -in server.ec.key -out server.ec.key.pem
openssl pkcs8 -topk8 -nocrypt -in client.ec.key -out client.ec.key.pem

mv ca.ec.key.pem ca.ec.key
mv server.ec.key.pem server.ec.key
mv client.ec.key.pem client.ec.key

openssl req -x509 -sha256 -days 365 -nodes -key ca.ec.key -config ca.ini -out ca.ec.crt
openssl req -x509 -sha256 -days 365 -CA ca.ec.crt -CAkey ca.ec.key -nodes -key server.ec.key -config server.ini -out server.ec.crt
openssl req -x509 -sha256 -days 365 -CA ca.ec.crt -CAkey ca.ec.key -nodes -key client.ec.key -config client.ini -out client.ec.crt
