import { useAuthStore } from './authStore';

export const apiFetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const token = useAuthStore.getState().getActiveToken();
    const headers = new Headers(init?.headers);
    const method = init?.method?.toUpperCase() || 'GET';   
    if (token && !headers.has('Authorization')) {
        headers.set('Authorization', `Bearer ${token}`);
    }
    if (method !== 'GET' && !headers.has('Content-Type')) {
        headers.set('Content-Type', 'application/json');
    }
    const response = await fetch(input, {
        ...init,
        headers,
    });
    if (response.status === 401) {
        const state = useAuthStore.getState();
        if (state.activeEmail) {
            state.removeAccount(state.activeEmail);
            if (window.location.pathname !== '/login') {
                window.location.reload();
            }
        }
    }
    return response;
}