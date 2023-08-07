# Wild
Umbrella project (a virtual cargo workspace) for a couple of wild animals.
The common denominator for these projects is that they are all clients for the Lipa backend GraphQL API.

## Honey-badger
The honey *badger* provides you with a badge that authorizes you to access the backend API.  
It deals with the backends authentication flow and leaves you with a JWT token
that you can use in your HTTP header as a bearer token.

## Chameleon
The chameleon can change its color very quickly, it knows everything about exchange rates.
So it will fetch fiat exchange rates for you.

## Mole
The mole lives within channels and knows everything about them.    
Therefore, it is the perfect agent to deal with a users channel states.  
It stores a users *channel manager* and *channel monitors* in encrypted form in the GraphQL backend
and always returns the latest state of them when requested.

*Caution: The library does not encrypt the data itself. It expects to retrieve the data in an already encrypted form.*

## Crow
Crows are known for their love to collect stuff.
The library allows to register for and list withdraw collect offers (e.g. Lightning Address).
