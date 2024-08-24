# define a new stack

region = 'eu-central-1'

# list of custom resources to monitor
custom_resources = [
    'CustomResourceFunction'
]
values = {
    "cidrs": ["10.10.10.0/24", "10.10.10.1/24"],
    "subnet": {
        "region": region
    }
}

# define a new stack
stack = stacks.new(
    name = 'example', # when interacting with the stack, this is the name that will be used
    region = region,
    template = os.open('./template.yaml'),
    values = values,
    capabilities = [
        'CAPABILITY_AUTO_EXPAND',
        'CAPABILITY_IAM'
    ],
    custom_resources = custom_resources
)

# add the stack to the list of stacks
stacks.add(stack)
