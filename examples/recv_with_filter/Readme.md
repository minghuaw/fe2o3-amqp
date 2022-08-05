# Receive with filter

This example will require a broker that is capable of message filtering. One could use the `activemq-artemis` [docker image](https://hub.docker.com/r/vromero/activemq-artemis) for such purpose.
A sender that sends messges with `"sn"` set to `100` is also needed to see the full effect. One could use the example [`send_with_custom_properties`](../send_with_custom_properties/).
