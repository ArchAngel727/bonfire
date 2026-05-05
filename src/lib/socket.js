import { io } from 'socket.io-client';

const URL = 'http://127.0.0.1:3000';

export const registerSocket = io(`${URL}/register`, { autoConnect: false });
export const loginSocket = io(`${URL}/login`, { autoConnect: false });
export const messageSocket = io(`${URL}/message`, { autoConnect: false });