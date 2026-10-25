export const SAMPLE_SWAGGER_SPEC: string = JSON.stringify(
  {
    openapi: '3.0.3',
    info: { title: 'Petstore API', version: '1.0.0' },
    servers: [{ url: '/api/v1' }],
    paths: {
      '/pets': {
        get: {
          tags: ['Pets'],
          operationId: 'listPets',
          summary: 'List all pets',
          responses: {
            '200': {
              description: 'A paged array of pets',
              content: {
                'application/json': {
                  example: [
                    { id: 1, name: 'Rex', tag: 'dog' },
                    { id: 2, name: 'Milo', tag: 'cat' }
                  ]
                }
              }
            }
          }
        },
        post: {
          tags: ['Pets'],
          operationId: 'createPet',
          summary: 'Create a pet',
          responses: {
            '201': {
              description: 'Created pet',
              content: {
                'application/json': {
                  example: { id: 3, name: 'Luna', tag: 'dog' }
                }
              }
            }
          }
        }
      },
      '/pets/{id}': {
        get: {
          tags: ['Pets'],
          operationId: 'getPetById',
          summary: 'Info for a specific pet',
          responses: {
            '200': {
              description: 'Expected response to a valid request',
              content: {
                'application/json': {
                  example: { id: '{{id}}', name: 'Rex', tag: 'dog' }
                }
              }
            },
            '404': { description: 'Pet not found' }
          }
        },
        delete: {
          tags: ['Pets'],
          operationId: 'deletePet',
          summary: 'Delete a pet',
          responses: {
            '204': { description: 'Pet deleted' }
          }
        }
      },
      '/store/orders': {
        get: {
          tags: ['Store/Orders'],
          operationId: 'listOrders',
          summary: 'List customer orders',
          responses: {
            '200': {
              description: 'Orders list',
              content: {
                'application/json': {
                  example: [{ orderId: 'ord_101', petId: 1, quantity: 1, status: 'placed' }]
                }
              }
            }
          }
        },
        post: {
          tags: ['Store/Orders'],
          operationId: 'placeOrder',
          summary: 'Place an order for a pet',
          responses: {
            '201': {
              description: 'Order placed',
              content: {
                'application/json': {
                  example: { orderId: 'ord_102', status: 'approved' }
                }
              }
            }
          }
        }
      },
      '/users/me': {
        get: {
          tags: ['Users'],
          operationId: 'getCurrentUser',
          summary: 'Get current signed-in user',
          responses: {
            '200': {
              description: 'Current user profile',
              content: {
                'application/json': {
                  example: { id: 'usr_1', username: 'alex', role: 'admin' }
                }
              }
            }
          }
        }
      }
    }
  },
  null,
  2
);
